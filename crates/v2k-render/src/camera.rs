/// FPS-style fly camera with yaw/pitch rotation.
pub struct Camera {
    pub position: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub fov: f32,
    pub near: f32,
    pub far: f32,
    pub aspect: f32,
    /// Off-centre projection in normalized device coordinates. The original
    /// V2000 renderer changes this per menu prop group instead of translating
    /// the models themselves.
    pub projection_offset: [f32; 2],
    /// Reflect the right-handed OpenGL view X axis to reproduce V2000's
    /// left-handed world presentation. In retail gameplay, a camera looking
    /// toward +Z keeps authored +X on screen-right; an ordinary OpenGL basis
    /// would put it on screen-left. Menu cameras leave this off. Live-world
    /// free-fly keeps it on so a chase detach does not mirror the world.
    pub left_handed: bool,
}

impl Camera {
    pub fn new(aspect: f32) -> Self {
        Self {
            position: [128.0, 30.0, 128.0],
            yaw: 0.0,
            pitch: -0.3,
            fov: 60.0_f32.to_radians(),
            near: 0.1,
            far: 500.0,
            aspect,
            projection_offset: [0.0, 0.0],
            left_handed: false,
        }
    }

    /// Forward direction vector (into the screen).
    pub fn forward(&self) -> [f32; 3] {
        let cp = self.pitch.cos();
        [self.yaw.sin() * cp, self.pitch.sin(), -self.yaw.cos() * cp]
    }

    /// Right direction vector (cross(forward, up) where up = [0,1,0]).
    pub fn right(&self) -> [f32; 3] {
        let fwd = self.forward();
        // cross([fx,fy,fz], [0,1,0]) = [-fz, 0, fx]
        let mut rx = -fwd[2];
        let mut rz = fwd[0];
        if self.left_handed {
            rx = -rx;
            rz = -rz;
        }
        let len = (rx * rx + rz * rz).sqrt();
        if len < 1e-6 {
            return [1.0, 0.0, 0.0];
        }
        [rx / len, 0.0, rz / len]
    }

    /// View matrix as column-major 4x4 (OpenGL convention).
    ///
    /// Builds a look-at matrix from position, yaw, and pitch.
    pub fn view_matrix(&self) -> [f32; 16] {
        let fwd = self.forward();
        let up = [0.0_f32, 1.0, 0.0];

        // right = normalize(cross(fwd, up))
        let rx = fwd[1] * up[2] - fwd[2] * up[1];
        let ry = fwd[2] * up[0] - fwd[0] * up[2];
        let rz = fwd[0] * up[1] - fwd[1] * up[0];
        let rlen = (rx * rx + ry * ry + rz * rz).sqrt();
        let (mut rx, mut ry, mut rz) = if rlen > 1e-6 {
            (rx / rlen, ry / rlen, rz / rlen)
        } else {
            (1.0, 0.0, 0.0)
        };

        // true_up = cross(right, fwd). Compute this from the ordinary
        // right-handed basis first, then reflect only view X when adapting
        // retail's left-handed gameplay camera. This keeps +Y upright.
        let ux = ry * fwd[2] - rz * fwd[1];
        let uy = rz * fwd[0] - rx * fwd[2];
        let uz = rx * fwd[1] - ry * fwd[0];
        if self.left_handed {
            rx = -rx;
            ry = -ry;
            rz = -rz;
        }

        let p = self.position;
        let tx = -(rx * p[0] + ry * p[1] + rz * p[2]);
        let ty = -(ux * p[0] + uy * p[1] + uz * p[2]);
        let tz = -(-fwd[0] * p[0] + -fwd[1] * p[1] + -fwd[2] * p[2]);

        // Column-major: OpenGL expects column-major layout
        // Row 0 = right, Row 1 = up, Row 2 = -forward
        [
            rx, ux, -fwd[0], 0.0, ry, uy, -fwd[1], 0.0, rz, uz, -fwd[2], 0.0, tx, ty, tz, 1.0,
        ]
    }

    /// Perspective projection matrix as column-major 4x4.
    pub fn projection_matrix(&self) -> [f32; 16] {
        let f = 1.0 / (self.fov / 2.0).tan();
        let nf = 1.0 / (self.near - self.far);

        [
            f / self.aspect,
            0.0,
            0.0,
            0.0,
            0.0,
            f,
            0.0,
            0.0,
            self.projection_offset[0],
            self.projection_offset[1],
            (self.far + self.near) * nf,
            -1.0,
            0.0,
            0.0,
            2.0 * self.far * self.near * nf,
            0.0,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forward_at_zero_yaw() {
        let cam = Camera::new(1.0);
        // At yaw=0, pitch=0 the camera looks down -Z
        let mut cam = cam;
        cam.pitch = 0.0;
        cam.yaw = 0.0;
        let fwd = cam.forward();
        assert!((fwd[0]).abs() < 1e-5);
        assert!((fwd[1]).abs() < 1e-5);
        assert!((fwd[2] + 1.0).abs() < 1e-5); // -Z
    }

    #[test]
    fn projection_not_nan() {
        let cam = Camera::new(800.0 / 600.0);
        let proj = cam.projection_matrix();
        for v in &proj {
            assert!(!v.is_nan());
        }
    }

    #[test]
    fn projection_offset_moves_the_optical_center() {
        let mut cam = Camera::new(4.0 / 3.0);
        cam.projection_offset[1] = 2.0 * 170.0 / 240.0 - 1.0;
        let proj = cam.projection_matrix();
        assert!((proj[9] - (2.0 * 170.0 / 240.0 - 1.0)).abs() < 1e-6);
    }

    #[test]
    fn view_not_nan() {
        let cam = Camera::new(800.0 / 600.0);
        let view = cam.view_matrix();
        for v in &view {
            assert!(!v.is_nan());
        }
    }

    #[test]
    fn left_handed_positive_z_camera_keeps_positive_x_on_screen_right() {
        let mut cam = Camera::new(4.0 / 3.0);
        cam.position = [0.0, 0.0, -8.0];
        cam.yaw = std::f32::consts::PI;
        cam.pitch = 0.0;
        cam.left_handed = true;

        let view = cam.view_matrix();
        let world = [1.0_f32, 0.0, 0.0];
        let view_x = view[0] * world[0] + view[4] * world[1] + view[8] * world[2] + view[12];
        let view_z = view[2] * world[0] + view[6] * world[1] + view[10] * world[2] + view[14];

        assert!(view_x > 0.0, "+X must remain on screen-right");
        assert!(view_z < 0.0, "the target must remain in front of OpenGL");
        assert!(cam.right()[0] > 0.0);
    }

    #[test]
    fn dropping_left_handed_mirrors_screen_x_at_the_same_yaw() {
        let mut cam = Camera::new(4.0 / 3.0);
        cam.position = [0.0, 0.0, -8.0];
        cam.yaw = std::f32::consts::PI;
        cam.pitch = 0.0;
        cam.left_handed = true;
        let left_handed_x = {
            let view = cam.view_matrix();
            view[0] + view[12]
        };
        cam.left_handed = false;
        let right_handed_x = {
            let view = cam.view_matrix();
            view[0] + view[12]
        };
        assert!(left_handed_x > 0.0);
        assert!(right_handed_x < 0.0);
    }
}
