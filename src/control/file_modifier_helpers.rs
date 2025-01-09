/*
 Prod
 Copyright 2021-2025 Peter Pearson.
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

// This is separated out in its own file partly because it didn't really fit
// anywhere else perfectly (common_actions_unix_edit_file.rs should really just
// be above the EditFile action, which maybe in the future once that's more
// capable and robust could be used instead of this), and also to allow easier
// unit testing of this functionality.

// This functionality is bespoke in order (given current limitations and robustness
// of existing EditFile functionality) to be more robust to variations like whitespace
// and comments.
// Technically-speaking, it is likely over-engineered, as for very basic
// functionality it's likely that the use of sed or awk might have sufficed on the assumption
// that in most cases, the stock sshd_config config values to modify will not have whitespace
// around particular items, however, I didn't want to rely on that, and by the
// same token, the existing EditFile functionality in Prod could also be used in limited 
// scenarios as well, but I really want this bespoke and native for the moment in order
// to be a bit more forgiving with regards to whitespace and block comment examples, and to
// provide more flexibility and control around this in the future if needed.

pub enum SshDPermitRootLoginType {
    No,
    ProhibitPassword,
    Yes
}

pub struct ModifySshDConfigParams {
    pub password_authentication:        Option<bool>,
    pub permit_empty_passwords:         Option<bool>,
    pub permit_root_login:              Option<SshDPermitRootLoginType>,
    pub port:                           Option<u16>,
    pub pub_key_authentication:         Option<bool>,
}

impl ModifySshDConfigParams {
    pub fn new() -> ModifySshDConfigParams {
        ModifySshDConfigParams {
            password_authentication: None,
            permit_empty_passwords: None,
            permit_root_login: None,
            port: None,
            pub_key_authentication: None }
    }

    /// returns if any have actually been set or not
    pub fn any_set(&self) -> bool {
        self.password_authentication.is_some() ||
        self.permit_empty_passwords.is_some() ||
        self.permit_root_login.is_some() ||
        self.port.is_some() ||
        self.pub_key_authentication.is_some()
    }
}

/// takes in the current string contents of the file, and if successful,
/// returns the modified string contents of the file to be re-saved again.
pub fn modify_sshd_config_file_contents(file_content: &str,
    params: &ModifySshDConfigParams) -> Option<String> {

    let mut lines: Vec<String> = file_content.lines().map(|line| line.to_string()).collect();

    // make any changes based on if param fields are set or not...

    if let Some(password_auth) = params.password_authentication {
        let new_val_str = if password_auth { "yes" } else { "no" };
        set_file_lines_param_value(&mut lines, "PasswordAuthentication", new_val_str);
    }

    if let Some(permit_empty_passwords) = params.permit_empty_passwords {
        let new_val_str = if permit_empty_passwords { "yes" } else { "no" };
        set_file_lines_param_value(&mut lines, "PermitEmptyPasswords", new_val_str);
    }

    if let Some(permit_root_login) = &params.permit_root_login {
        let new_val_str = match permit_root_login {
            SshDPermitRootLoginType::No => "no",
            SshDPermitRootLoginType::ProhibitPassword => "prohibit-password",
            SshDPermitRootLoginType::Yes => "yes",
        };
        set_file_lines_param_value(&mut lines, "PermitRootLogin", new_val_str);
    }

    if let Some(port) = params.port {
        let new_val_str = format!("{}", port);
        set_file_lines_param_value(&mut lines, "Port", &new_val_str);
    }

    if let Some(pub_key_auth) = params.pub_key_authentication {
        let new_val_str = if pub_key_auth { "yes" } else { "no" };
        set_file_lines_param_value(&mut lines, "PubkeyAuthentication", new_val_str);
    }

    // convert back to single string for entire file, and make sure we append a newline on the end...
    let final_content = lines.join("\n") + "\n";
    Some(final_content)
}

// do the actual modification per key value.
// brute-force
fn set_file_lines_param_value(file_content_lines: &mut Vec<String>,
    key: &str,
    value: &str) -> bool {

    // this keeps track of a possible candidate line to insert (as a new line)
    // the new param string value at (above this one), and tries to look for existing
    // values or commented-out example values of the same param key...
    let mut candidate_insert_line: Option<usize> = None;

    // whether the value is already set to this, so we don't have to do anything
    // TODO: implement this to skip making changes if not needed...
//    let mut is_set_already = false;

    // do a single pass, finding an ideal line insert index for the new value
    // (we'll add a new line for the new value, rather than replace),
    // and commenting out any existing uncommented specifications of that
    // param key we find

    for (idx, line) in file_content_lines.iter_mut().enumerate() {
        let key_pos = line.find(key);
        if key_pos.is_none() {
            continue;
        }

        // we've found the param key string on this line 
        let key_pos = key_pos.unwrap();

        // first of all, make sure it is the correct whole string, and we're not
        // just finding a substring
        let remainder_string = &line[key_pos + key.len()..];
        // check if next chars are whitespace
        let next_non_whitespace = remainder_string.chars().position(|c| !" \t".contains(c));
        if let Some(next_non_whitespace_pos) = next_non_whitespace {
            if next_non_whitespace_pos == 0 {
                // we're not interested, as this isn't the 'key' value we care about...
                continue;
            }
        }

        let is_commented;

        // now try and work out if it's commented out or not
        if key_pos > 0 {
            // there's something before it at least in terms of characters
            // (but could be whitespace, which we also need to ignore)

            let before_string = &line[..key_pos];
            // strip away any whitespace
            let before_string_trimmed = before_string.trim();

            // check if it's a comment char in front
            if before_string_trimmed.starts_with('#') {
                // if it's just a single comment char, we can skip modifying this line,
                // as it's already commented
                is_commented = true;

                // if we haven't seen a candidate insert line before, use this one
                if candidate_insert_line.is_none() {
                    candidate_insert_line = Some(idx);
                }
            }
            else {
                // otherwise, this is very likely an existing statement setting the param key
                // to the old value
                is_commented = false;
            }
        }
        else {
            // there's no leading substring
            is_commented = false;
        }

        // if we reach here and it's not commented, for the moment, we can comment this line out
        // as it's the old value, and we don't want it to be applicable any more...
        if !is_commented {
            // TODO: check what the old value is: it might be correct...
            line.insert(0, '#');
            candidate_insert_line = Some(idx);
        }
    }

    // if we have a candidate insert line pos, insert the new line above that pos
    // with the new value set
    if let Some(new_line_pos) = candidate_insert_line {
        file_content_lines.insert(new_line_pos, format!("{} {}", key, value));
    }
    else {
        // if we didn't have a candidate insert line, for the moment (probably want
        // to rethink this, and maybe make it configurable), insert it at the very
        // top of the file, just so we at least do set it always.
        file_content_lines.insert(0, format!("{} {}", key, value));
    }
    
    // TODO: We can't really "fail" currently, so maybe remove this, or return something
    // more useful like if we updated/commented or inserted...
    true
}

// test stub to do the equivalent just for unit tests
#[cfg(test)]
fn test_modify_helper(input_content: &str,
    param_key: &str,
    param_value: &str) -> String {

    let mut lines: Vec<String> = input_content.lines().map(|line| line.to_string()).collect();

    set_file_lines_param_value(&mut lines, param_key, param_value);

    let final_content = lines.join("\n");
    final_content
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_param_value_1() {

const INPUT1: &str =
"something else
myParam oldVal
Something else again";

const EXPECTED1: &str = 
"something else
myParam newVal
#myParam oldVal
Something else again";

    assert_eq!(test_modify_helper(INPUT1, "myParam", "newVal"), EXPECTED1.to_string());

const INPUT2: &str =
"something else
myParamSkip oldVal
myParam oldVal
Something else again";

const EXPECTED2: &str = 
"something else
myParamSkip oldVal
myParam newVal
#myParam oldVal
Something else again";

    assert_eq!(test_modify_helper(INPUT2, "myParam", "newVal"), EXPECTED2.to_string());

const INPUT3: &str =
"something else
myParamSkip oldVal
#myParam oldVal1
Something else again";

const EXPECTED3: &str = 
"something else
myParamSkip oldVal
myParam newVal
#myParam oldVal1
Something else again";

    assert_eq!(test_modify_helper(INPUT3, "myParam", "newVal"), EXPECTED3.to_string());
        

const INPUT4: &str =
"something else
myParamSkip oldVal
#myParam oldVal1
myParam oldVal2
Something else again";

const EXPECTED4: &str = 
"something else
myParamSkip oldVal
#myParam oldVal1
myParam newVal2
#myParam oldVal2
Something else again";

    assert_eq!(test_modify_helper(INPUT4, "myParam", "newVal2"), EXPECTED4.to_string());

// now test yet again with indented leading whitespace...

    const INPUT5: &str =
    "something else
    myParamSkip oldVal
    #myParam oldVal1
    myParam oldVal2
    Something else again";

    const EXPECTED5: &str = 
    "something else
    myParamSkip oldVal
    #myParam oldVal1
myParam newVal2
#    myParam oldVal2
    Something else again";

    assert_eq!(test_modify_helper(INPUT5, "myParam", "newVal2"), EXPECTED5.to_string());
        
    }
}