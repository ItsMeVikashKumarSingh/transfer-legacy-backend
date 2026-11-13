#[cfg(test)]
mod tests {
    use crate::jcs::canonicalize;
    use serde::Serialize;

    #[derive(Serialize)]
    struct PayloadA {
        z_index: u32,
        alpha: String,
        count: u64,
    }

    #[derive(Serialize)]
    struct PayloadB {
        count: u64,
        alpha: String,
        z_index: u32,
    }

    #[test]
    fn test_jcs_canonicalize_key_order_invariance() {
        let a = PayloadA {
            z_index: 10,
            alpha: "hello".to_string(),
            count: 42,
        };
        let b = PayloadB {
            count: 42,
            alpha: "hello".to_string(),
            z_index: 10,
        };

        let canonical_a = canonicalize(&a).expect("canonicalize A");
        let canonical_b = canonicalize(&b).expect("canonicalize B");

        assert_eq!(canonical_a, canonical_b);
        assert_eq!(
            String::from_utf8(canonical_a).unwrap(),
            r#"{"alpha":"hello","count":42,"z_index":10}"#
        );
    }
}
