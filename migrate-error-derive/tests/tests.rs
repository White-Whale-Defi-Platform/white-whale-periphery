extern crate migrate_error_derive;
extern crate semver;

use migrate_error_derive::migrate_invalid_version_error;
use semver::Version;
use thiserror::Error;

#[test]
fn test_migrate_invalid_version_error_macro() {
    // Define the enum before macro application
    #[migrate_invalid_version_error]
    #[derive(Error, Debug)]
    pub enum TestError {
        #[error("{0}")]
        Std(String),
    }

    // Create instances of the enum variants
    let std_error = TestError::Std("Standard error".into());
    let migrate_error = TestError::MigrateInvalidVersion {
        new_version: Version::parse("2.0.0").unwrap(),
        current_version: Version::parse("1.0.0").unwrap(),
    };

    // Test if the error messages are formatted correctly
    assert_eq!(format!("{}", std_error), "Standard error");
    assert_eq!(
        format!("{}", migrate_error),
        "Attempt to migrate to version 2.0.0, but contract is on a higher version 1.0.0"
    );
}

#[test]
fn test_macro_preserves_attributes() {
    #[migrate_invalid_version_error]
    #[derive(Error, Debug, PartialEq)]
    pub enum AnotherError {
        #[error("{0}")]
        Std(String),
    }

    // Ensure that #[derive(PartialEq)] works correctly
    let error1 = AnotherError::Std("Error".into());
    let error2 = AnotherError::Std("Error".into());
    assert_eq!(error1, error2);
}
