# frozen_string_literal: true

require 'fiddle'
require 'fiddle/import'
require 'json'

module Natsuzora
  module FFI
    extend Fiddle::Importer

    LIB_PATH = ENV.fetch('NATSUZORA_LIB_PATH') do
      ext_dir = File.expand_path('../../ext', __dir__)
      lib_name = RUBY_PLATFORM.include?('darwin') ? 'libnatsuzora_ffi.dylib' : 'libnatsuzora_ffi.so'
      File.join(ext_dir, lib_name)
    end

    dlload LIB_PATH

    extern 'void* nz_render_json(const char*, const char*, const char*, void*)'
    extern 'void nz_string_free(void*)'

    POINTER_PACK_FORMAT = (Fiddle::SIZEOF_VOIDP == 8 ? 'Q' : 'L')

    module_function

    def render(source, json_data, include_root)
      err_buf = Fiddle::Pointer.malloc(Fiddle::SIZEOF_VOIDP)
      err_buf[0, Fiddle::SIZEOF_VOIDP] = [0].pack(POINTER_PACK_FORMAT)

      result_ptr = nz_render_json(source, json_data, include_root, err_buf)

      if result_ptr.null?
        err_addr = err_buf[0, Fiddle::SIZEOF_VOIDP].unpack1(POINTER_PACK_FORMAT)
        err_ptr = Fiddle::Pointer.new(err_addr)
        begin
          error_json = err_ptr.to_s
        ensure
          nz_string_free(err_ptr)
        end
        raise_from_error_json(error_json)
      end

      begin
        result_ptr.to_s.force_encoding(Encoding::UTF_8)
      ensure
        nz_string_free(result_ptr)
      end
    ensure
      err_buf&.free
    end

    ERROR_TYPE_MAP = {
      'ParseError' => ParseError,
      'UndefinedVariable' => UndefinedVariableError,
      'NullValueError' => RenderError,
      'EmptyStringError' => RenderError,
      'TypeError' => TypeError,
      'IncludeError' => IncludeError,
      'ShadowingError' => ShadowingError,
      'IoError' => IncludeError
    }.freeze

    def raise_from_error_json(json_str)
      info = JSON.parse(json_str)
      klass = ERROR_TYPE_MAP[info['type']] || RenderError
      raise klass.new(info['message'], line: info['line'], column: info['column'])
    end
  end
end
