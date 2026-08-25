/// Fixed distance between the machine reference point and the Gerber2Gcode origin.
pub const REFERENCE_OFFSET_MM: f64 = 3.064;

#[derive(Debug, Clone, Copy, Default)]
pub struct ToolOffsetInputs {
    pub indicator_x: f64,
    pub indicator_y: f64,
    pub clamp_x: f64,
    pub clamp_y: f64,
    pub safety_x: f64,
    pub safety_y: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ToolOffsetResults {
    pub program_zero_x: f64,
    pub program_zero_y: f64,
    pub program_home_x: f64,
    pub program_home_y: f64,
    pub material_offset_x: f64,
    pub material_offset_y: f64,
    pub material_edge_margin_x: f64,
    pub material_edge_margin_y: f64,
}

impl ToolOffsetInputs {
    pub fn calculate(self) -> ToolOffsetResults {
        let material_offset = |indicator| indicator;
        let edge_margin = |clamp, safety| clamp + safety + REFERENCE_OFFSET_MM;

        let material_offset_x = material_offset(self.indicator_x);
        let material_offset_y = material_offset(self.indicator_y);
        let material_edge_margin_x = edge_margin(self.clamp_x, self.safety_x);
        let material_edge_margin_y = edge_margin(self.clamp_y, self.safety_y);

        ToolOffsetResults {
            program_zero_x: material_offset_x + material_edge_margin_x,
            program_zero_y: material_offset_y + material_edge_margin_y,
            program_home_x: REFERENCE_OFFSET_MM,
            program_home_y: REFERENCE_OFFSET_MM,
            material_offset_x,
            material_offset_y,
            material_edge_margin_x,
            material_edge_margin_y,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculates_gerber_and_application_offsets() {
        let results = ToolOffsetInputs {
            indicator_x: 20.0,
            indicator_y: 30.0,
            clamp_x: 4.0,
            clamp_y: 5.0,
            safety_x: 2.0,
            safety_y: 3.0,
        }
        .calculate();

        assert_eq!(results.material_offset_x, 20.0);
        assert_eq!(results.material_offset_y, 30.0);
        assert!((results.material_edge_margin_x - 9.064).abs() < 0.000_001);
        assert!((results.material_edge_margin_y - 11.064).abs() < 0.000_001);
        assert!((results.program_zero_x - 29.064).abs() < 0.000_001);
        assert!((results.program_zero_y - 41.064).abs() < 0.000_001);
        assert_eq!(results.program_home_x, REFERENCE_OFFSET_MM);
        assert_eq!(results.program_home_y, REFERENCE_OFFSET_MM);
    }
}
