pub const VERTEX_SHADER_SRC: &str = r#"
        #version 140

        uniform mat4 matrix;

        in vec2 position;
        in vec2 tex_coords;

        out vec2 v_tex_coords;
                
        void main() {
            v_tex_coords = tex_coords;
            gl_Position = matrix * vec4(position, 0.0, 1.0);
        }
    "#;

pub const FRAGMENT_SHADER_SRC: &str = r#"
        #version 140

        uniform sampler2D tex;

        in vec2 v_tex_coords;
        out vec4 color;
        
        void main() {
            color = texture(tex, v_tex_coords);
        }
    "#;

#[derive(Copy, Clone)]
pub struct Vertex {
    pub position: [f32; 2],
    pub tex_coords: [f32; 2],
}
implement_vertex!(Vertex, position, tex_coords);
