#version 330 core

in vec2 f_uv;
in vec4 f_color;

out vec4 frag_color;

uniform sampler2D tex;  //Not _necessarily_ a font atlas

void main() {
    frag_color = f_color * texture(tex, f_uv);
}