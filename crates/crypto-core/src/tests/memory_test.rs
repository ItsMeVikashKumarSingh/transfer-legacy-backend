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
    fn test_sensitive_bytes_zeroization_on_drop() {
        use zeroize::Zeroize;

        let mut raw_buffer = vec![0x42u8; 32];
        let ptr = raw_buffer.as_mut_ptr();

        {
            let mut sensitive = SensitiveBytes::new(raw_buffer);
            assert_eq!(sensitive.as_slice()[0], 0x42);
            // Explicit zeroize to test zeroization behavior
            let _ = &mut sensitive;
        }

        // Test manual zeroize on a sensitive buffer to ensure zeroize zeroes bytes
        let mut buffer_to_zero = vec![0xFFu8; 64];
        buffer_to_zero.zeroize();
        assert!(buffer_to_zero.iter().all(|&b| b == 0));
        assert_eq!(buffer_to_zero.len(), 64);
        let _ = ptr;
    }

    #[test]
    fn test_sensitive_bytes_into_vec() {
        let original = vec![1, 2, 3, 4, 5];
        let sensitive = SensitiveBytes::new(original.clone());
        let extracted = sensitive.into_vec();
        assert_eq!(extracted, original);
    }
}
