#[cfg(test)]
mod tests {
    use crate::memory::SensitiveBytes;

    #[test]
    fn test_sensitive_bytes_creation_and_slice() {
        let secret = vec![0xDE, 0xAD, 0xBE, 0xEF];
        let sensitive = SensitiveBytes::new(secret.clone());
        assert_eq!(sensitive.as_slice(), secret.as_slice());
    }

    #[test]
    fn test_sensitive_bytes_zeroization_behavior() {
        use zeroize::Zeroize;

        // Test slice zeroization
        let mut buffer = [0xFFu8; 32];
        buffer.as_mut_slice().zeroize();
        assert!(buffer.iter().all(|&b| b == 0));
        assert_eq!(buffer.len(), 32);

        // Test SensitiveBytes drop handling
        let sensitive = SensitiveBytes::new(vec![0x42u8; 32]);
        assert_eq!(sensitive.as_slice()[0], 0x42);
        drop(sensitive);
    }

    #[test]
    fn test_sensitive_bytes_into_vec() {
        let original = vec![1, 2, 3, 4, 5];
        let sensitive = SensitiveBytes::new(original.clone());
        let extracted = sensitive.into_vec();
        assert_eq!(extracted, original);
    }
}
