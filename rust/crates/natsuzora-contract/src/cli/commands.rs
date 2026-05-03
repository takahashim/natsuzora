use anyhow::{anyhow, Context, Result};
use std::{
    collections::{BTreeMap, HashSet},
    fs,
    path::PathBuf,
};
use natsuzora_contract::{
    check_template, extract_contract, validate, write, Contract, ContractField, ContractFile,
    ContractFileWithDiff, TypeDef,
};

use super::loader::FileIncludeLoader;
use super::project::{
    collect_ntzc_files, collect_ntzr_files, resolve_dirs, resolve_include_root,
};
use super::{
    ApplyArgs, CheckArgs, ExtractArgs, OutputFormat, ParseArgs, SyncArgs, ValidateArgs,
};

pub(super) fn run_extract(args: ExtractArgs) -> Result<()> {
    let include_root = resolve_include_root(args.include_root.as_deref(), &args.template);

    let source =
        fs::read_to_string(&args.template).with_context(|| format!("reading {:?}", args.template))?;
    let template =
        natsuzora_ast::parse(&source).map_err(|err| anyhow!("parse error: {err}"))?;

    let mut loader = FileIncludeLoader::new(include_root);
    let contract = extract_contract(&template, &mut loader)?;

    if let Some(data_path) = args.data.as_ref() {
        let data_text =
            fs::read_to_string(data_path).with_context(|| format!("reading {data_path:?}"))?;
        let data_json: serde_json::Value = serde_json::from_str(&data_text)
            .with_context(|| format!("parsing JSON {data_path:?}"))?;
        validate(&contract, &data_json)
            .map_err(|err| anyhow!("validation failed: {err}"))?;
        eprintln!("Validation passed");
    }

    output_contract(&contract, &args.format, args.output.as_ref())
}

pub(super) fn run_check(args: CheckArgs) -> Result<()> {
    let is_batch = match &args.path {
        Some(p) if p.is_dir() => true,
        Some(p) if p.extension().and_then(|e| e.to_str()) == Some("ntzr") => false,
        None if args.templates_dir.is_some() || args.contracts_dir.is_some() => true,
        None => false,
        Some(_) => true, // no extension or unknown → directory (batch) mode
    };

    if is_batch {
        run_check_batch(args)
    } else {
        run_check_single(args)
    }
}

fn run_check_single(args: CheckArgs) -> Result<()> {
    if args.templates_dir.is_some() || args.contracts_dir.is_some() {
        return Err(anyhow!(
            "--templates-dir / --contracts-dir cannot be used with a single template file"
        ));
    }

    let template_path = args
        .path
        .ok_or_else(|| anyhow!("template file path is required for single-file mode"))?;
    let contract_path = args
        .contract
        .ok_or_else(|| anyhow!("--contract is required for single-file mode"))?;

    let include_root = resolve_include_root(args.include_root.as_deref(), &template_path);

    let contract_source = fs::read_to_string(&contract_path)
        .with_context(|| format!("reading {contract_path:?}"))?;
    let contract = natsuzora_contract::parse(&contract_source).map_err(|e| anyhow!("{e}"))?;

    let template_source = fs::read_to_string(&template_path)
        .with_context(|| format!("reading {template_path:?}"))?;
    let template = natsuzora_ast::parse(&template_source)
        .map_err(|err| anyhow!("parse error: {err}"))?;

    let mut loader = FileIncludeLoader::new(include_root);
    let errors = check_template(&template, &contract, &mut loader);

    if errors.is_empty() {
        eprintln!("Template check passed: no violations found");
        return Ok(());
    }

    let file_name = template_path.display();
    for error in &errors {
        eprintln!(
            "{}:{}:{}: error: {}",
            file_name, error.location.line, error.location.column, error.message
        );
        if let Some(suggestion) = &error.suggestion {
            eprintln!("  hint: {suggestion}");
        }
    }
    Err(anyhow!("{} violation(s) found", errors.len()))
}

fn run_check_batch(args: CheckArgs) -> Result<()> {
    let base = args.path.as_deref();
    let (templates_dir, contracts_dir) =
        resolve_dirs(base, args.templates_dir.as_deref(), args.contracts_dir.as_deref())?;

    if !templates_dir.exists() {
        return Err(anyhow!("templates directory not found: {templates_dir:?}"));
    }
    if !contracts_dir.exists() {
        return Err(anyhow!("contracts directory not found: {contracts_dir:?}"));
    }

    let include_root = args
        .include_root
        .unwrap_or_else(|| templates_dir.clone());

    let templates = collect_ntzr_files(&templates_dir, args.name.as_deref())?;
    let contracts = collect_ntzc_files(&contracts_dir, None)?;
    let contract_names: HashSet<&str> = contracts.iter().map(|(n, _)| n.as_str()).collect();

    if templates.is_empty() {
        eprintln!("No templates found");
        return Ok(());
    }

    let mut passed = 0usize;
    let mut failed = 0usize;
    let mut skipped = 0usize;
    let mut total_violations = 0usize;

    for (name, template_path) in &templates {
        if !contract_names.contains(name.as_str()) {
            eprintln!("{name}: skip (no contract)");
            skipped += 1;
            continue;
        }

        let contract_path = contracts_dir.join(format!("{name}.ntzc"));

        let contract_source = match fs::read_to_string(&contract_path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("{name}: error reading contract: {e}");
                failed += 1;
                continue;
            }
        };
        let contract = match natsuzora_contract::parse(&contract_source) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("{name}: error parsing contract: {e}");
                failed += 1;
                continue;
            }
        };

        let template_source = match fs::read_to_string(template_path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("{name}: error reading template: {e}");
                failed += 1;
                continue;
            }
        };
        let template = match natsuzora_ast::parse(&template_source) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("{name}: error parsing template: {e}");
                failed += 1;
                continue;
            }
        };

        let mut loader = FileIncludeLoader::new(include_root.clone());
        let errors = check_template(&template, &contract, &mut loader);

        if errors.is_empty() {
            eprintln!("{name}: ok");
            passed += 1;
        } else {
            let file_name = template_path.display();
            for error in &errors {
                eprintln!(
                    "{}:{}:{}: error: {}",
                    file_name, error.location.line, error.location.column, error.message
                );
                if let Some(suggestion) = &error.suggestion {
                    eprintln!("  hint: {suggestion}");
                }
            }
            total_violations += errors.len();
            failed += 1;
        }
    }

    eprintln!();
    eprintln!(
        "Checked {} template(s), {} passed, {} failed, {} skipped ({} violation(s)).",
        passed + failed,
        passed,
        failed,
        skipped,
        total_violations
    );

    if failed > 0 || skipped > 0 {
        Err(anyhow!(
            "{failed} template(s) failed check, {skipped} skipped (no contract)"
        ))
    } else {
        Ok(())
    }
}

pub(super) fn run_validate(args: ValidateArgs) -> Result<()> {
    let contract_source = fs::read_to_string(&args.contract)
        .with_context(|| format!("reading {:?}", args.contract))?;
    let contract =
        natsuzora_contract::parse(&contract_source).map_err(|e| anyhow!("{e}"))?;

    let data_text =
        fs::read_to_string(&args.data).with_context(|| format!("reading {:?}", args.data))?;
    let data_json: serde_json::Value = serde_json::from_str(&data_text)
        .with_context(|| format!("parsing JSON {:?}", args.data))?;

    validate(&contract, &data_json).map_err(|err| anyhow!("validation failed: {err}"))?;
    eprintln!("Validation passed");
    Ok(())
}

pub(super) fn run_parse(args: ParseArgs) -> Result<()> {
    let source = fs::read_to_string(&args.contract)
        .with_context(|| format!("reading {:?}", args.contract))?;
    let contract =
        natsuzora_contract::parse(&source).map_err(|e| anyhow!("{e}"))?;
    output_contract(&contract, &args.format, args.output.as_ref())
}

pub(super) fn run_sync(args: SyncArgs) -> Result<()> {
    let (templates_dir, contracts_dir) = resolve_dirs(
        args.path.as_deref(),
        args.templates_dir.as_deref(),
        args.contracts_dir.as_deref(),
    )?;

    let include_root = args.include_root.unwrap_or_else(|| templates_dir.clone());

    let templates = collect_ntzr_files(&templates_dir, args.name.as_deref())?;

    if templates.is_empty() {
        eprintln!("No templates found");
        return Ok(());
    }

    let mut synced = 0usize;
    let mut created = 0usize;
    let mut unchanged = 0usize;

    for (name, template_path) in &templates {
        let mut loader = FileIncludeLoader::new(include_root.clone());

        let template_source = fs::read_to_string(template_path)
            .with_context(|| format!("reading {template_path:?}"))?;
        let template = natsuzora_ast::parse(&template_source)
            .map_err(|err| anyhow!("parse error in {template_path:?}: {err}"))?;

        let extracted = extract_contract(&template, &mut loader)?;

        let contract_path = contracts_dir.join(format!("{name}.ntzc"));

        if contract_path.exists() {
            let existing_source = fs::read_to_string(&contract_path)
                .with_context(|| format!("reading {contract_path:?}"))?;
            let existing =
                natsuzora_contract::parse(&existing_source).map_err(|e| anyhow!("{e}"))?;

            let diff = natsuzora_contract::diff_contracts(&existing, &extracted);

            if natsuzora_contract::has_changes(&diff) {
                eprintln!("{name}:");
                eprint!("{}", natsuzora_contract::format_diff(&diff));

                if !args.dry_run {
                    if let Some(parent) = contract_path.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    fs::write(&contract_path, natsuzora_contract::write_with_diff(&diff))?;
                }
                synced += 1;
            } else {
                eprintln!("{name}: ok");
                unchanged += 1;
            }
        } else {
            eprintln!("{name}: new");

            if !args.dry_run {
                if let Some(parent) = contract_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                fs::write(&contract_path, write(&extracted))?;
            }
            created += 1;
        }
    }

    eprintln!();
    eprintln!(
        "sync: {synced} synced, {created} created, {unchanged} unchanged"
    );
    Ok(())
}

pub(super) fn run_apply(args: ApplyArgs) -> Result<()> {
    let contracts_dir = match (args.contracts_dir, args.path) {
        (Some(c), _) => c,
        (None, Some(b)) => b.join("contracts"),
        (None, None) => {
            return Err(anyhow!(
                "project directory or -C/--contracts-dir is required"
            ))
        }
    };

    let contracts = collect_ntzc_files(&contracts_dir, args.name.as_deref())?;

    if contracts.is_empty() {
        eprintln!("No contracts found");
        return Ok(());
    }

    let mut applied = 0usize;
    let mut skipped = 0usize;

    for (name, contract_path) in &contracts {
        let source = fs::read_to_string(contract_path)
            .with_context(|| format!("reading {contract_path:?}"))?;
        let parsed =
            natsuzora_contract::parse_file_with_diff(&source).map_err(|e| anyhow!("{e}"))?;

        if natsuzora_contract::has_changes(&parsed) {
            let next = natsuzora_contract::apply_diff(&parsed);
            fs::write(contract_path, write_contract_file(&next))?;
            eprintln!("{name}: applied");
            applied += 1;
        } else {
            eprintln!("{name}: skip (no markers)");
            skipped += 1;
        }
    }

    eprintln!();
    eprintln!("apply: {applied} applied, {skipped} skipped");
    Ok(())
}

fn output_contract(
    contract: &Contract,
    format: &OutputFormat,
    output_path: Option<&PathBuf>,
) -> Result<()> {
    let content = match format {
        OutputFormat::Contract => write(contract),
        OutputFormat::Json => serde_json::to_string_pretty(contract)?,
    };

    if let Some(path) = output_path {
        fs::write(path, &content).with_context(|| format!("writing {path:?}"))?;
    } else {
        print!("{content}");
    }
    Ok(())
}

/// Write a ContractFile (without diff markers) to notation string.
fn write_contract_file(file: &ContractFile) -> String {
    let types = file
        .types
        .iter()
        .map(|(k, v)| (k.clone(), TypeDef::new(v.clone())))
        .collect();

    let fields = match &file.root {
        Contract::Object { properties, .. } => properties
            .iter()
            .map(|(k, v)| (k.clone(), ContractField::new(v.clone())))
            .collect(),
        _ => BTreeMap::new(),
    };

    let diff_file = ContractFileWithDiff { types, fields };
    natsuzora_contract::write_with_diff(&diff_file)
}
