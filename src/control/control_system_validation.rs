/*
 Prod
 Copyright 2021-2024 Peter Pearson.
 Licensed under the Apache License, Version 2.0 (the "License");
 You may not use this file except in compliance with the License.
 You may obtain a copy of the License at
 http://www.apache.org/licenses/LICENSE-2.0
 Unless required by applicable law or agreed to in writing, software
 distributed under the License is distributed on an "AS IS" BASIS,
 WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 See the License for the specific language governing permissions and
 limitations under the License.
 ---------
*/

// release version - done as string, so we can cope with
// both single integer versions and number.number versions.
#[derive(Clone, Debug, PartialEq)]
pub enum SystemValidationReleaseVersion {
    None,
    Equal(String),
    LessThan(String),
    LessThanOrEqual(String),
    GreaterThan(String),
    GreaterThanOrEqual(String)
}

impl SystemValidationReleaseVersion {
    fn is_version_okay(&self, host_version: &str) -> bool {

        // TODO: some of this validation is probably better done when reading the action script file rather than here?

        // get the actual string
        if let Some(expected_release_string) = self.get_version_string() {
            // if it has a dot in it, we don't currently support that yet...
            // TODO: handle . dot notation for things like Ubuntu...
            if expected_release_string.contains('.') {
                eprintln!("Error: SystemValidationReleaseVersion doesn't currently support release verions with '.' chars in.");
                return false;
            }

            // otherwise, assume the expected value is an integer...
            let expected_int_version = expected_release_string.parse::<u32>();
            if let Ok(expected_int_version_value) = expected_int_version {
                // now get the actual host version.

                // again, we don't support . dot notation yet
                if host_version.contains('.') {
                    eprintln!("Error: SystemValidationReleaseVersion doesn't currently support release verions with '.' chars in.");
                    return false;
                }

                let actual_int_version = host_version.parse::<u32>();
                if let Ok(actual_int_version_value) = actual_int_version {
                    // we now have both integer values, so check them

                    let is_valid = match *self {
                        SystemValidationReleaseVersion::LessThan(_) => {
                            actual_int_version_value < expected_int_version_value
                        },
                        SystemValidationReleaseVersion::LessThanOrEqual(_) => {
                            actual_int_version_value <= expected_int_version_value
                        },
                        SystemValidationReleaseVersion::Equal(_) => {
                            actual_int_version_value == expected_int_version_value
                        },
                        SystemValidationReleaseVersion::GreaterThan(_) => {
                            actual_int_version_value > expected_int_version_value
                        },
                        SystemValidationReleaseVersion::GreaterThanOrEqual(_) => {
                            actual_int_version_value >= expected_int_version_value
                        },
                        _  => {
                            // shouldn't be possible to reach here, but...
                            false
                        }
                    };

                    return is_valid;
                }
                else {
                    eprintln!("Error parsing SystemValidationReleaseVersion actual version string: '{}'", host_version);
                    return false;
                }
            }
            else {
                // error
                eprintln!("Error parsing SystemValidationReleaseVersion expected version string: '{}'", expected_release_string);
                return false;
            }
        }
        else {
            // otherwise, no release constraint version was set, so just return true
            return true;
        }
    }

    fn get_version_string(&self) -> Option<String> {
        match self {
            SystemValidationReleaseVersion::None                        => None,
            SystemValidationReleaseVersion::LessThan(value)             => Some(value.clone()),
            SystemValidationReleaseVersion::LessThanOrEqual(value)      => Some(value.clone()),
            SystemValidationReleaseVersion::Equal(value)                => Some(value.clone()),
            SystemValidationReleaseVersion::GreaterThan(value)          => Some(value.clone()),
            SystemValidationReleaseVersion::GreaterThanOrEqual(value)   => Some(value.clone()),
        }
    }
}

#[derive(Clone, Debug)]
pub struct SystemValidation {
    // optional id/name string to validate the system against (i.e. "Debian")
    pub id_name:       Option<String>,

    // release number validation - note: as strings
    pub release:       SystemValidationReleaseVersion,

    // TODO: maybe more stuff like validating free disk space and amount
    //       of memory in the future?
}

// System validation infrastructure. This might need a bit of a re-think, but it currently
// does provide the ability to verify that a system matches particular specifics, i.e.
// it's Debian 12 and not 11 for example...
impl SystemValidation {
    pub fn new() -> SystemValidation {
        SystemValidation { id_name: None, release: SystemValidationReleaseVersion::None }
    }

    // whether this is enabled/set or not and so needs checking
    pub fn needs_checking(&self) -> bool {
        return self.id_name.is_some() ||
               self.release != SystemValidationReleaseVersion::None;
    }

    // check the actual values from the host and see if they pass the
    // configured check
    pub fn check_actual_distro_values(&self, distro_id: &str, release: &str) -> bool {
        // check the distributor id name if wanted...
        if let Some(expected_id_name) = &self.id_name {
            // do a case-insensitive comparison for the moment...
            if !expected_id_name.eq_ignore_ascii_case(distro_id) {
                // distributor name/id didn't match...
                return false;
            }
        }

        // check the release version   
        if !self.release.is_version_okay(release) {
            // version was not acceptable...
            return false;
        }
        
        // otherwise, assume things are okay...
        true
    }

    fn process_value(&mut self, value: &str) -> bool {
        // first of all, see if there are any numbers in the string, by finding the first
        // char position of any number
        if let Some(first_number_pos) = value.chars().position(|c| c.is_numeric()) {
            // there were numbers, so assume for the moment, this string is the release/version,
            // (i.e. "12", or "20.04").

            // see if we might have a prefix
            if first_number_pos != 0 {
                // if first_number_pos != 0, assume we've got a custom comparison operator prefix first
                let prefix = &value[0..first_number_pos];
                let remainder_value = &value[first_number_pos..];
                // check the remainder value after the comparison operator string is not empty
                if remainder_value.is_empty() {
                    eprintln!("Error: missing SystemValidation release version value string.");
                    return false;
                }
                // otherwise, parse
                match prefix {
                    "="  => {
                        self.release = SystemValidationReleaseVersion::Equal(remainder_value.to_string());
                    },
                    "<"  => {
                        self.release = SystemValidationReleaseVersion::LessThan(remainder_value.to_string());
                    },
                    "<=" => {
                        self.release = SystemValidationReleaseVersion::LessThanOrEqual(remainder_value.to_string());
                    },
                    ">"  => {
                        self.release = SystemValidationReleaseVersion::GreaterThan(remainder_value.to_string());
                    },
                    ">=" => {
                        self.release = SystemValidationReleaseVersion::GreaterThanOrEqual(remainder_value.to_string());
                    },
                    _  => {
                        eprintln!("Error: unsupported SystemValidation release version operator.");
                        return false;
                    }
                }
            }
            else {
                // if there's no prefix, we assume it's just equals comparison
                self.release = SystemValidationReleaseVersion::Equal(value.to_string());
            }
        }
        else {
            // otherwise, if it hasn't got any number chars in, assume it's the distribution name/id

            // check it is a valid non-empty string
            if value.is_empty() {
                eprintln!("Error: missing SystemValidation release version value string.");
                return false;
            }

            // also check there's at least some alphabetic chars in there, as it could just be release version
            // comparison operator chars without any numbers...
            if !value.chars().any(|c| c.is_alphabetic()) {
                // we didn't find any alphabetic chars, so it can't be a valid distribution name/id
                eprintln!("Error: invalid SystemValidation value: '{}'", value);
                return false;
            }

            self.id_name = Some(value.to_string());
        }

        true
    }

    pub fn parse_string_value(value: &str) -> Result<SystemValidation, String> {
        // we attempt to be "clever" here, in the interests of "convenience",
        // but not sure how great an idea that is...

        // the assumption is the string will either be:
        // 1. a single value - which could be either the distro name (i.e. "Debian")
        //    to validate against, or the distro release number (i.e. "11" or "12"
        //    for Debian, or "20.04" for Ubuntu) to validate against.
        // 2. Multiple values separated by a comma, one of which is the distro name/id and
        //    the other is the release number. This code will attempt to work out which
        //    is which.

        // Note: this doesn't support checking for things like codenames, i.e. "bookworm",
        //       because that's a whole other dimension of somewhat arbitrary strings to deal
        //       with.

        // see if it's a tuple in parenthesis first...
        let working_value;
        if let Some(clipped_val) = value.strip_prefix('(') {
            // assume ther should be a closing ')'
            if let Some(extracted_val) = clipped_val.strip_suffix(')') {
                working_value = extracted_val;
            }
            else {
                return Err("Invalid SystemValidation string: missing closing parenthesis".to_string());
            }
        }
        else {
            // otherwise, just the whole value
            working_value = value;
        }

        if working_value.is_empty() {
            return Err("Invalid SystemValidation string: empty value".to_string());           
        }

        let mut parsed_values = SystemValidation::new();

        // if there's a comma, we have two values
        if let Some((value1, value2)) = working_value.split_once(',') {
            // also trim any whitespace, just to be a bit flexible...
            if !parsed_values.process_value(value1.trim()) {
                return Err(format!("Invalid SystemValidation string: couldn't interpret string value correctly: '{}'", value1));
            }
            if !parsed_values.process_value(value2.trim()) {
                return Err(format!("Invalid SystemValidation string: couldn't interpret string value correctly: '{}'", value2));
            }
        }
        else {
            // otherwise, we only have one value, so try and work out what it is...
            if !parsed_values.process_value(working_value) {
                return Err(format!("Invalid SystemValidation string: couldn't interpret string value correctly: '{}'", working_value));
            }
        }

        Ok(parsed_values)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_invalid_strings1() {

        assert_eq!(SystemValidation::parse_string_value("").is_err(), true);

        assert_eq!(SystemValidation::parse_string_value("(").is_err(), true);

        assert_eq!(SystemValidation::parse_string_value("()").is_err(), true);

        assert_eq!(SystemValidation::parse_string_value("(egeg").is_err(), true);
    }

    #[test]
    fn test_parse_single_string() {

        let res1 = SystemValidation::parse_string_value("Debian");
        assert!(res1.is_ok());
        if let Ok(internal) = res1 {
            assert_eq!(internal.id_name, Some("Debian".to_string()));
            assert_eq!(internal.release, SystemValidationReleaseVersion::None);
        }

        let res2 = SystemValidation::parse_string_value("12");
        assert!(res2.is_ok());
        if let Ok(internal) = res2 {
            assert!(internal.id_name.is_none());
            assert_eq!(internal.release, SystemValidationReleaseVersion::Equal("12".to_string()));
        }

        let res3 = SystemValidation::parse_string_value("20.04");
        assert!(res3.is_ok());
        if let Ok(internal) = res3 {
            assert!(internal.id_name.is_none());
            assert_eq!(internal.release, SystemValidationReleaseVersion::Equal("20.04".to_string()));
        }

        let res4 = SystemValidation::parse_string_value("<12");
        assert!(res4.is_ok());
        if let Ok(internal) = res4 {
            assert!(internal.id_name.is_none());
            assert_eq!(internal.release, SystemValidationReleaseVersion::LessThan("12".to_string()));
        }

        // test unsupported comparison operator string gets caught as an error
        let res5 = SystemValidation::parse_string_value("?12");
        assert!(res5.is_err());

        // test just a comparison operator gets caught as an error
        let res6 = SystemValidation::parse_string_value(">=");
        assert!(res6.is_err());
    }

    #[test]
    fn test_parse_multiple_strings_parens1() {

        let res1 = SystemValidation::parse_string_value("(Debian,12)");
        assert!(res1.is_ok());
        if let Ok(internal) = res1 {
            assert_eq!(internal.id_name, Some("Debian".to_string()));
            assert_eq!(internal.release, SystemValidationReleaseVersion::Equal("12".to_string()));
        }

        let res2 = SystemValidation::parse_string_value("(12,Debian)");
        assert!(res2.is_ok());
        if let Ok(internal) = res2 {
            assert_eq!(internal.id_name, Some("Debian".to_string()));
            assert_eq!(internal.release, SystemValidationReleaseVersion::Equal("12".to_string()));
        }

        let res3 = SystemValidation::parse_string_value("(>=12,Debian)");
        assert!(res3.is_ok());
        if let Ok(internal) = res3 {
            assert_eq!(internal.id_name, Some("Debian".to_string()));
            assert_eq!(internal.release, SystemValidationReleaseVersion::GreaterThanOrEqual("12".to_string()));
        }

        // check invalid release version without a value gets caught as an error.
        let res4 = SystemValidation::parse_string_value("(>=,Debian)");
        assert!(res4.is_err());
    }

    #[test]
    fn test_version_comparisons_single_int1() {
        if let Ok(internal) = SystemValidation::parse_string_value("<12") {
            assert!(internal.release.is_version_okay("11"));
            assert!(!internal.release.is_version_okay("12"));
        }
        else {
            assert!(false);
        }

        if let Ok(internal) = SystemValidation::parse_string_value("<=12") {
            assert!(internal.release.is_version_okay("11"));
            assert!(internal.release.is_version_okay("12"));
        }
        else {
            assert!(false);
        }
        
    }
}