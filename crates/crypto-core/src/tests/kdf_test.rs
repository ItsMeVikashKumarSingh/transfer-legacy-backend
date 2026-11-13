#[cfg(test)]
mod tests {
    use crate::kdf::Argon2Params;

    #[test]
    fn test_valid_desktop_and_mobile_defaults() {
        let desktop = Argon2Params::desktop_defaults("random_salt_b64_string".to_string());
        assert!(desktop.validate().is_ok());
        assert_eq!(desktop.m, 262144);
        assert_eq!(desktop.t, 2);
        assert_eq!(desktop.p, 2);

        let mobile = Argon2Params::mobile_defaults("random_salt_b64_string".to_string());
        assert!(mobile.validate().is_ok());
        assert_eq!(mobile.m, 65536);
        assert_eq!(mobile.t, 2);
        assert_eq!(mobile.p, 1);
    }

    #[test]
    fn test_argon2_memory_parameter_too_low() {
        let mut params = Argon2Params::desktop_defaults("salt".to_string());
        params.m = 1024; // Less than 16384 minimum
        let result = params.validate();
        assert_eq!(result.unwrap_err(), "memory parameter 'm' too low");
    }

    #[test]
    fn test_argon2_iterations_parameter_too_low() {
        let mut params = Argon2Params::desktop_defaults("salt".to_string());
        params.t = 0; // Less than 1
        let result = params.validate();
        assert_eq!(result.unwrap_err(), "iterations parameter 't' must be >= 1");
    }

    #[test]
    fn test_argon2_parallelism_parameter_too_low() {
        let mut params = Argon2Params::desktop_defaults("salt".to_string());
        params.p = 0; // Less than 1
        let result = params.validate();
        assert_eq!(
            result.unwrap_err(),
            "parallelism parameter 'p' must be >= 1"
        );
    }

    #[test]
    fn test_argon2_empty_salt_rejected() {
        let mut params = Argon2Params::desktop_defaults("".to_string());
        params.salt = "".to_string();
        let result = params.validate();
        assert_eq!(result.unwrap_err(), "salt cannot be empty");
    }
}
