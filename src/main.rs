use colorful::{Color, Colorful};

unsafe fn link_program<C: glow::HasContext>(gl: &C, vs: &str, ps: &str) -> C::Program {
    let program = gl.create_program().unwrap();
    let shader_sources = [(glow::VERTEX_SHADER, vs), (glow::FRAGMENT_SHADER, ps)];
    for (shader_type, shader_source) in shader_sources.iter() {
        let shader = gl.create_shader(*shader_type).unwrap();
        gl.shader_source(shader, shader_source);
        gl.compile_shader(shader);
        if !gl.get_shader_compile_status(shader) {
            panic!(gl.get_shader_info_log(shader));
        }
        gl.attach_shader(program, shader);
    }
    gl.link_program(program);
    if !gl.get_program_link_status(program) {
        panic!(gl.get_program_info_log(program));
    }
    program
}

fn report(name: &str, actual: [u8; 4], expected: [u8; 4]) {
    print!("\t{}: ", name.color(Color::LightBlue));
    if actual == expected {
        println!("{}", "PASS".color(Color::Green));
    } else {
        println!("{} {:?}", "FAIL".color(Color::Red), actual);
    }
}

/// Swizzle test.
///
/// Setting up the swizzle on a texture unit may corrupt the shader metadata,
/// making the `textureSize` return garbage from the shader.
///
/// Affected: Intel 4000 GPU on macOS
fn test_swizzle<C: glow::HasContext>(gl: &C, extensions: &[String]) {
    println!("Test: {}", "swizzle".color(Color::Blue));
    print!("\tRelevant extensions:");
    for ext in extensions {
        if ext.contains("swizzle") {
            print!(" {}", ext.as_str().color(Color::Yellow));
        }
    }
    println!("");

    let vs_shader = r#"#version 330 core
        uniform sampler2DArray tex;
        out vec4 color;
        void main() {
            vec2 tc = vec2(ivec2(gl_VertexID/2, gl_VertexID%2));
            vec2 tex_size = vec2(textureSize(tex, 0).xy);
            color = vec4(tex_size/64.0, 0.0, 1.0);
            gl_Position = vec4(2.0*tc - 1.0, 0.0, 1.0);
        }
    "#;
    let fs_shader = r#"#version 330 core
        precision mediump float;
        in vec4 color;
        out vec4 o_Color;
        void main() {
            o_Color = color;
        }
    "#;

    let mut texel = [0u8; 4];
    unsafe {
        // initialize the texture data
        let size = 64;
        let texture = gl.create_texture().unwrap();
        gl.bind_texture(glow::TEXTURE_2D_ARRAY, Some(texture));
        gl.tex_storage_3d(glow::TEXTURE_2D_ARRAY, 1, glow::RGBA8, size, size, 2);
        gl.tex_parameter_i32(
            glow::TEXTURE_2D_ARRAY,
            glow::TEXTURE_SWIZZLE_R,
            glow::BLUE as i32,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D_ARRAY,
            glow::TEXTURE_SWIZZLE_G,
            glow::GREEN as i32,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D_ARRAY,
            glow::TEXTURE_SWIZZLE_B,
            glow::RED as i32,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D_ARRAY,
            glow::TEXTURE_SWIZZLE_A,
            glow::ALPHA as i32,
        );

        // link the program reads the texture size from the VS
        let program = link_program(gl, vs_shader, fs_shader);

        // prepare to draw
        let draw_size = 256;
        let vertex_array = gl.create_vertex_array().unwrap();
        gl.bind_vertex_array(Some(vertex_array));
        let renderbuf = gl.create_renderbuffer().unwrap();
        gl.bind_renderbuffer(glow::RENDERBUFFER, Some(renderbuf));
        gl.renderbuffer_storage(glow::RENDERBUFFER, glow::RGBA8, draw_size, draw_size);
        let framebuf = gl.create_framebuffer().unwrap();
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(framebuf));
        gl.framebuffer_renderbuffer(
            glow::FRAMEBUFFER,
            glow::COLOR_ATTACHMENT0,
            glow::RENDERBUFFER,
            Some(renderbuf),
        );

        // draw a quad
        gl.clear(glow::COLOR_BUFFER_BIT);
        gl.viewport(0, 0, draw_size, draw_size);
        gl.use_program(Some(program));
        gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);

        // read back a texel
        gl.read_pixels(
            size / 2,
            size / 2,
            1,
            1,
            glow::RGBA,
            glow::UNSIGNED_BYTE,
            &mut texel,
        );
    };

    report("textureSize", texel, [0xFF, 0xFF, 0, 0xFF]);
}

fn test_pbo_upload<C: glow::HasContext>(gl: &C) {
    println!("Test: {}", "PBO uploads".color(Color::Blue));
    const FILL: [u8; 4] = [0, 0xFF, 0, 0xFF];
    let size = (256, 16);
    let mut texels = [[0u8; 4]; 3];
    unsafe {
        // initialize the texture
        let texture = gl.create_texture().unwrap();
        gl.bind_texture(glow::TEXTURE_2D_ARRAY, Some(texture));
        gl.tex_storage_3d(glow::TEXTURE_2D_ARRAY, 1, glow::RGBA8, size.0, size.1, 2);
        gl.tex_sub_image_3d_u8_slice(
            glow::TEXTURE_2D_ARRAY,
            0,
            0,
            0,
            0,
            size.0,
            size.1,
            2,
            glow::RGBA,
            glow::UNSIGNED_BYTE,
            Some(&vec![0xFF; (size.0 as usize) * (size.1 as usize) * 4]),
        );

        // initialize a framebuffer for reading pixels back
        let framebuf = gl.create_framebuffer().unwrap();
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(framebuf));
        gl.framebuffer_texture_layer(
            glow::FRAMEBUFFER,
            glow::COLOR_ATTACHMENT0,
            Some(texture),
            0,
            0,
        );
        gl.read_pixels(0, 0, 1, 1, glow::RGBA, glow::UNSIGNED_BYTE, &mut texels[0]);

        // create a pixel buffer with one line
        let pixelbuf = gl.create_buffer().unwrap();
        gl.bind_buffer(glow::PIXEL_UNPACK_BUFFER, Some(pixelbuf));
        gl.buffer_data_u8_slice(
            glow::PIXEL_UNPACK_BUFFER,
            &(0..size.0)
                .flat_map(|_| FILL.iter().cloned())
                .collect::<Vec<_>>(),
            glow::STATIC_DRAW,
        );
        let texel_offset = 16;
        let pbo_offset = 128 * 4;
        gl.tex_sub_image_3d_pixel_buffer_offset(
            glow::TEXTURE_2D_ARRAY,
            0,
            texel_offset,
            0,
            0,
            16,
            1,
            1,
            glow::RGBA,
            glow::UNSIGNED_BYTE,
            pbo_offset,
        );
        gl.read_pixels(
            texel_offset,
            0,
            1,
            1,
            glow::RGBA,
            glow::UNSIGNED_BYTE,
            &mut texels[1],
        );

        // upload to the other layer of the texture
        gl.tex_sub_image_3d_pixel_buffer_offset(
            glow::TEXTURE_2D_ARRAY,
            0,
            0,
            0,
            1,
            16,
            1,
            1,
            glow::RGBA,
            glow::UNSIGNED_BYTE,
            0,
        );
        gl.framebuffer_texture_layer(
            glow::FRAMEBUFFER,
            glow::COLOR_ATTACHMENT0,
            Some(texture),
            0,
            1,
        );
        gl.read_pixels(0, 0, 1, 1, glow::RGBA, glow::UNSIGNED_BYTE, &mut texels[2]);
    }

    assert_eq!(texels[0], [0xFF; 4]);
    report("offset-128px", texels[1], FILL);
    report("layer-1", texels[2], FILL);
}

fn main() {
    use glow::HasContext;

    // this snippet allows us to use the GPU selected by "gfxCardStatus" tool
    #[cfg(target_os = "macos")]
    {
        use core_foundation::{self as cf, base::TCFType};
        let i = cf::bundle::CFBundle::main_bundle().info_dictionary();
        let mut i = unsafe { i.to_mutable() };
        i.set(
            cf::string::CFString::new("NSSupportsAutomaticGraphicsSwitching"),
            cf::boolean::CFBoolean::true_value().into_CFType(),
        );
    }

    let connection = surfman::Connection::new().unwrap();
    let adapter = connection.create_adapter().unwrap();
    let mut device = connection.create_device(&adapter).unwrap();
    let context_attributes = surfman::ContextAttributes {
        version: surfman::GLVersion::new(3, 3),
        flags: surfman::ContextAttributeFlags::empty(),
    };
    let context_descriptor = device
        .create_context_descriptor(&context_attributes)
        .unwrap();
    let mut context = device.create_context(&context_descriptor).unwrap();
    let gl = glow::Context::from_loader_function(|s| device.get_proc_address(&context, s));
    device.make_context_current(&context).unwrap();

    let renderer = unsafe { gl.get_parameter_string(glow::RENDERER) };
    let extensions = unsafe {
        let num = gl.get_parameter_i32(glow::NUM_EXTENSIONS);
        (0..num)
            .map(|i| gl.get_parameter_indexed_string(glow::EXTENSIONS, i as u32))
            .collect::<Vec<_>>()
    };
    let has_khr_debug = extensions.iter().any(|ext| ext.as_str() == "GL_KHR_debug");
    println!("Init with renderer: {}", renderer.color(Color::Violet));

    test_swizzle(&gl, &extensions);
    test_pbo_upload(&gl);

    if has_khr_debug {
        let debug_messages = unsafe { gl.get_debug_message_log(10) };
        if !debug_messages.is_empty() {
            println!("Debug messages:");
            for msg in debug_messages {
                println!("\t{:?}", msg);
            }
        }
    } else {
        let error = unsafe { gl.get_error() };
        if error != glow::NO_ERROR {
            println!("Last {}: {:?}", "ERROR".color(Color::Red), error);
        }
    }

    device.destroy_context(&mut context).unwrap();
    println!("Done");
}
