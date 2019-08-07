use glow::{Context, RenderLoop};

fn main() {
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

    let (gl, mut events_loop, render_loop) = {
        use glutin::GlContext;
        let events_loop = glutin::EventsLoop::new();
        let window_builder = glutin::WindowBuilder::new()
            .with_title("GL Swizzle Test")
            .with_dimensions(glutin::dpi::LogicalSize::new(1024.0, 768.0));
        let context_builder = glutin::ContextBuilder::new().with_vsync(true);
        let window =
            glutin::GlWindow::new(window_builder, context_builder, &events_loop).unwrap();
        unsafe {
            window.make_current().unwrap();
        }
        let context = glow::native::Context::from_loader_function(|s| {
            window.get_proc_address(s) as *const _
        });
        let render_loop =
            glow::native::RenderLoop::<glutin::GlWindow>::from_glutin_window(window);
        (context, events_loop, render_loop)
    };

    let renderer = unsafe {
         gl.get_parameter_string(glow::RENDERER)
    };
    let extensions = unsafe {
        let num = gl.get_parameter_i32(glow::NUM_EXTENSIONS);
        (0 .. num)
            .map(|i| {
                gl.get_parameter_indexed_string(glow::EXTENSIONS, i as u32)
            })
            .filter(|ext| ext.contains("swizzle"))
            .collect::<Vec<_>>()
    };
    println!("Renderer: {}", renderer);
    println!("Swizzle: {:?}", extensions);

    unsafe {
        let texture = gl.create_texture().unwrap();
        gl.bind_texture(glow::TEXTURE_2D_ARRAY, Some(texture));
        gl.tex_storage_3d(glow::TEXTURE_2D_ARRAY, 1, glow::RGBA8, 2, 1, 1);
        gl.tex_sub_image_3d_u8_slice(glow::TEXTURE_2D_ARRAY, 0, 0, 0, 0,
            2, 1, 1, glow::RGBA, glow::UNSIGNED_BYTE, Some(&[
                0xFF, 0, 0, 0xFF,
                0, 0, 0xFF, 0xFF,
            ]));
        gl.tex_parameter_i32(glow::TEXTURE_2D_ARRAY, glow::TEXTURE_MAG_FILTER, glow::LINEAR as i32);
        gl.tex_parameter_i32(glow::TEXTURE_2D_ARRAY, glow::TEXTURE_SWIZZLE_R, glow::BLUE as i32);
        gl.tex_parameter_i32(glow::TEXTURE_2D_ARRAY, glow::TEXTURE_SWIZZLE_B, glow::RED as i32);
    };

    unsafe {
        let vertex_array = gl
            .create_vertex_array()
            .expect("Cannot create vertex array");
        gl.bind_vertex_array(Some(vertex_array));
    }

    let program = unsafe {
        let program = gl.create_program().expect("Cannot create program");
        let vertex_shader_source = r#"#version 330 core
            const vec2 verts[3] = vec2[3](
                vec2(0.5f, 1.0f),
                vec2(0.0f, 0.0f),
                vec2(1.0f, 0.0f)
            );
            out vec2 tc;
            void main() {
                tc = verts[gl_VertexID];
                gl_Position = vec4(2.0*tc - 1.0, 0.0, 1.0);
            }
        "#;
        let fragment_shader_source = r#"#version 330 core
            precision mediump float;
            uniform sampler2DArray tex;
            in vec2 tc;
            out vec4 color;
            void main() {
                color = texture(tex, vec3(tc, 0.0));
            }
        "#;
        let shader_sources = [
            (glow::VERTEX_SHADER, vertex_shader_source),
            (glow::FRAGMENT_SHADER, fragment_shader_source),
        ];
        for (shader_type, shader_source) in shader_sources.iter() {
            let shader = gl
                .create_shader(*shader_type)
                .expect("Cannot create shader");
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
    };

    render_loop.run(move |running: &mut bool| {
        events_loop.poll_events(|event| {
            if let glutin::Event::WindowEvent { event, .. } = event {
                if let glutin::WindowEvent::CloseRequested = event {
                    *running = false;
                }
            }
        });

        unsafe {
            gl.clear(glow::COLOR_BUFFER_BIT);
            gl.use_program(Some(program));
            gl.draw_arrays(glow::TRIANGLES, 0, 3);
        }
    });
}
