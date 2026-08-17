pub mod cultures;
pub mod provinces;
pub mod religions;
pub mod river;
pub mod states;

#[cfg(test)]
mod tests {
    use crate::river::meander::meander_anchors;
    use crate::river::specify::{get_approximate_length, get_next_id};
    use crate::river::width::{get_offset, get_source_width, get_width, rn};
    use vor_core::entities::river::River;

    #[test]
    fn test_width_formulas_simple() {
        let offset = get_offset(500.0, 10, 1.0, 0.1);
        assert!(offset > 0.0);
        let width = get_width(offset);
        assert!(width > 0.0);
        let sw = get_source_width(500.0);
        assert!(sw > 0.0);
    }

    #[test]
    fn test_width_increases_with_flux() {
        let o1 = get_offset(100.0, 10, 1.0, 0.1);
        let o2 = get_offset(1000.0, 10, 1.0, 0.1);
        assert!(o2 > o1);
    }

    #[test]
    fn test_width_increases_with_length() {
        let o1 = get_offset(500.0, 5, 1.0, 0.1);
        let o2 = get_offset(500.0, 50, 1.0, 0.1);
        assert!(o2 > o1);
    }

    #[test]
    fn test_source_width_zero_flux() {
        let sw = get_source_width(0.0);
        assert_eq!(sw, 0.0);
    }

    #[test]
    fn test_get_offset_starting_point() {
        let o = get_offset(1000.0, 0, 1.0, 0.5);
        assert_eq!(o, 0.5);
    }

    #[test]
    fn test_meander_two_points() {
        let anchors = vec![[0.0, 0.0], [100.0, 0.0]];
        let is_water = vec![false, false];
        let result = meander_anchors(&anchors, &is_water);
        assert!(result.len() >= 2);
    }

    #[test]
    fn test_meander_single_point_is_unchanged() {
        let anchors = vec![[10.0, 20.0]];
        let is_water = vec![false];
        let result = meander_anchors(&anchors, &is_water);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], [10.0, 20.0]);
    }

    #[test]
    fn test_get_approximate_length_zero() {
        assert_eq!(get_approximate_length(&[]), 0.0);
        assert_eq!(get_approximate_length(&[[0.0, 0.0]]), 0.0);
    }

    #[test]
    fn test_get_approximate_length_known() {
        let pts = vec![[0.0, 0.0], [3.0, 0.0], [3.0, 4.0]];
        let len = get_approximate_length(&pts);
        assert!((len - 7.0).abs() < 0.001);
    }

    #[test]
    fn test_rn_rounding() {
        assert_eq!(rn(1.234, 2), 1.23);
        assert_eq!(rn(1.235, 2), 1.24); // JS Math.round ties-to-+inf
        assert_eq!(rn(1.236, 2), 1.24);
    }

    #[test]
    fn test_get_next_id_empty() {
        assert_eq!(get_next_id(&[]), 1);
    }

    #[test]
    fn test_get_next_id_with_existing() {
        let rivers = vec![
            River {
                id: 1,
                ..Default::default()
            },
            River {
                id: 5,
                ..Default::default()
            },
        ];
        assert_eq!(get_next_id(&rivers), 6);
    }
}
