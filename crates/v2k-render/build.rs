use gl_generator::{Api, Fallbacks, GlobalGenerator, Profile, Registry};
use std::env;
use std::fs::File;
use std::path::Path;

fn main() {
    let dest = env::var("OUT_DIR").unwrap();
    let mut file = File::create(Path::new(&dest).join("gl_bindings.rs")).unwrap();

    // Generate OpenGL 2.1 compatibility profile bindings.
    // This includes all the fixed-function pipeline functions:
    // glBegin/glEnd, glVertex*, glColor*, glMatrixMode, glLoadMatrix*,
    // glPushMatrix/glPopMatrix, glTranslate*, glRotate*, glFog*, etc.
    Registry::new(
        Api::Gl,
        (2, 1),
        Profile::Compatibility,
        Fallbacks::All,
        ["GL_ARB_framebuffer_object", "GL_EXT_framebuffer_object"],
    )
    .write_bindings(GlobalGenerator, &mut file)
    .unwrap();
}
