/*
 Prod
 Copyright 2021 Peter Pearson.
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

use crate::params::{ParamValue, Params};

use super::control_actions::{ActionProvider, ActionResult, ControlAction};
use super::control_common::{ControlConnection};
use super::terminal_helpers_linux;

use std::collections::BTreeMap;
use std::path::Path;
use std::io::prelude::*;

use rpassword::read_password;

pub struct AProviderLinuxDebian {
    
}

impl AProviderLinuxDebian {
    pub fn new() -> AProviderLinuxDebian {
        AProviderLinuxDebian {  }
    }
}

impl ActionProvider for AProviderLinuxDebian {
    fn name(&self) -> String {
        return "linux_debian".to_string();
    }

    fn add_user(&self, connection: &mut ControlConnection, params: &ControlAction) -> ActionResult {
        // validate params
        if !params.params.has_value("username") || !params.params.has_value("password") {
            return ActionResult::InvalidParams;
        }

        let mut useradd_command_options = String::new();

        let user = params.params.get_string_value("username").unwrap();
        let mut password = params.params.get_string_value("password").unwrap();
        if password == "$PROMPT" {
            eprintln!("Please enter password to set for user:");
            password = read_password().unwrap();
        }

        let create_home = params.params.get_value_as_bool("createHome", true);

        if create_home {
            useradd_command_options.push_str("-m ");
        }
        else {
            // do not create user's home group...
            useradd_command_options.push_str("-M ");
        }

        let shell = params.params.get_string_value_with_default("shell", "/bin/bash");
        useradd_command_options.push_str(&format!("-s {}", shell));

        let useradd_full_command = format!(" useradd {} {}", useradd_command_options, user);

        connection.conn.send_command(&useradd_full_command);

        // check response is nothing...
        if connection.conn.had_response() {
            return ActionResult::Failed("Unexpected response from useradd command.".to_string());
        }

//        let change_password_command = format!(" echo -e \"{0}\n{0}\" | passwd {1}", password, user);
        let change_password_command = format!(" echo -e '{}:{}' | chpasswd", user, password);
        connection.conn.send_command(&change_password_command);

        // now add user to any groups
        // see if there's just a single group...
        if params.params.has_value("group") {
            let usermod_command = format!(" usermod -aG {} {}", params.params.get_string_value_with_default("group", ""), user);
            connection.conn.send_command(&usermod_command);
        }
        else if params.params.has_value("groups") {
            // there's multiple
            let groups = params.params.get_values_as_vec_of_strings("groups");
            for group in groups {
                let usermod_command = format!(" usermod -aG {} {}", group, user);
                connection.conn.send_command(&usermod_command);
            }
        }

        eprintln!("Added user okay.");

        return ActionResult::Success;
    }

    fn create_directory(&self, connection: &mut ControlConnection, params: &ControlAction) -> ActionResult {
        // validate params
        if !params.params.has_value("path") {
            return ActionResult::InvalidParams;
        }

        let path_to_create = params.params.get_string_value("path").unwrap();
        let mnkdir_command = format!(" mkdir {}", path_to_create);
        connection.conn.send_command(&mnkdir_command);

        if let Some(permissions) = params.params.get_string_or_int_value_as_string("permissions") {
            let chmod_command = format!(" chmod {} {}", permissions, path_to_create);
            connection.conn.send_command(&chmod_command);
        }

        if let Some(owner) = params.params.get_string_value("owner") {
            let chown_command = format!(" chown {} {}", owner, path_to_create);
            connection.conn.send_command(&chown_command);
        }

        if let Some(group) = params.params.get_string_value("group") {
            let chgrp_command = format!(" chgrp {} {}", group, path_to_create);
            connection.conn.send_command(&chgrp_command);
        }

        // TODO: check for 'groups' as well to handle setting multiple...

        return ActionResult::Success;
    }

    fn install_packages(&self, connection: &mut ControlConnection, params: &ControlAction) -> ActionResult {
        // use apt-get, because the commands for that will apparently be much more stable, compared to apt
        // which might change as it's designed to be more user-facing...

        let packages_string;
        if let Some(package) = params.params.get_string_value("package") {
            // single package for convenience...
            packages_string = package;
        }
        else if params.params.has_value("packages") {
            let packages = params.params.get_values_as_vec_of_strings("packages");
            packages_string = packages.join(" ");
        }
        else {
            return ActionResult::InvalidParams;
        }

        if packages_string.is_empty() {
            return ActionResult::InvalidParams;
        }

        let apt_get_command = format!(" apt-get -y install {}", packages_string);
        connection.conn.send_command(&apt_get_command);

        return ActionResult::Success;
    }

    fn systemctrl(&self, connection: &mut ControlConnection, params: &ControlAction) -> ActionResult {
        // validate params
        if !params.params.has_value("action") || !params.params.has_value("service") {
            return ActionResult::InvalidParams;
        }

        let service = params.params.get_string_value("service").unwrap();
        let action = params.params.get_string_value("action").unwrap();

        let systemctrl_command = format!("systemctl {} {}", action, service);
        
        connection.conn.send_command(&systemctrl_command);

        return ActionResult::Success;
    }

    fn firewall(&self, connection: &mut ControlConnection, params: &ControlAction) -> ActionResult {
        let firewall_type = params.params.get_string_value_with_default("type", "ufw");
        if firewall_type != "ufw" {
            // only support this type for the moment...
            return ActionResult::InvalidParams;
        }

        // incredibly basic for the moment...
        // in theory we should probably be more type-specific, and 'schema'd', but given there
        // are aliases for rules, it'd be quite complicated to handle that I think, so better
        // for the moment to allow freeform strings...
        let rules = params.params.get_values_as_vec_of_strings("rules");
        for rule in rules {
            let ufw_command = format!(" ufw {}", rule);
            connection.conn.send_command(&ufw_command);
        }

        if params.params.has_value("enabled") {
            let is_enabled = params.params.get_value_as_bool("enabled", true);
            let ufw_command = format!(" ufw --force {}", if is_enabled { "enable" } else { "disable"});
            connection.conn.send_command(&ufw_command);
        }

        return ActionResult::Success;
    }

    fn edit_file(&self, connection: &mut ControlConnection, params: &ControlAction) -> ActionResult {
        let filepath = params.params.get_string_value("filepath");
        if filepath.is_none() {
            return ActionResult::InvalidParams;
        }

        let replace_line_items = extract_replace_line_entry_items_from_params_map(&params.params, "replaceLine");
        if replace_line_items.is_empty() {
            eprintln!("Error: editFile Control Action had no items to perform...");
            return ActionResult::InvalidParams;
        }

        let filepath = filepath.unwrap();
        
        if params.params.get_value_as_bool("backup", false) {
            // TODO: something more robust than this...
            let mv_command = format!(" cp {0} {0}.bak", filepath);
            connection.conn.send_command(&mv_command);
        }

        // Note: the Stat returned by scp_recv() is currently a private field, so we can only access bits of it,
        //       so we need to do a full stat call remotely to get the actual info
        let stat_command = format!(" stat {}", filepath);
        connection.conn.send_command(&stat_command);

        let stat_response = connection.conn.prev_std_out.clone();
        // get the details from the stat call...
        let stat_details = terminal_helpers_linux::extract_details_from_stat_output(&stat_response);

        // download the file
        let (mut remote_file, _stat) = connection.conn.session.scp_recv(Path::new(&filepath)).unwrap();

        let mut contents = Vec::new();
        remote_file.read_to_end(&mut contents).unwrap();

        // Close the channel and wait for the whole content to be tranferred
        remote_file.send_eof().unwrap();
        remote_file.wait_eof().unwrap();
        remote_file.close().unwrap();
        remote_file.wait_close().unwrap();

        let string_contents = String::from_utf8_lossy(&contents);
        let file_contents_lines = string_contents.lines();

        // brute force replacement...

        let mut new_file_contents_lines = Vec::new();
        for line in file_contents_lines {
            let mut have_replaced = false;

            for replace_item in &replace_line_items {
                if replace_item.match_type == ReplaceLineMatchType::Contains {
                    if line.contains(&replace_item.match_string) {
                        new_file_contents_lines.push(replace_item.replace_string.clone());
                        have_replaced = true;
                    }
                }
                else if replace_item.match_type == ReplaceLineMatchType::StartsWith {
                    if line.starts_with(&replace_item.match_string) {
                        new_file_contents_lines.push(replace_item.replace_string.clone());
                        have_replaced = true;
                    }
                }
                else if replace_item.match_type == ReplaceLineMatchType::EndsWith {
                    if line.ends_with(&replace_item.match_string) {
                        new_file_contents_lines.push(replace_item.replace_string.clone());
                        have_replaced = true;
                    }
                }
            }

            if !have_replaced {
                new_file_contents_lines.push(line.to_string());
            }
        }

        let new_file_contents_string = new_file_contents_lines.join("\n");
        let byte_contents = new_file_contents_string.as_bytes();

        // send the file back via upload

        let stat_details = stat_details.unwrap();

        let mode = i32::from_str_radix(&stat_details.0, 8).unwrap();

        let mut remote_file = connection.conn.session.scp_send(Path::new(&filepath), mode, byte_contents.len() as u64, None).unwrap();
        remote_file.write(byte_contents).unwrap();
        // Close the channel and wait for the whole content to be tranferred
        remote_file.send_eof().unwrap();
        remote_file.wait_eof().unwrap();
        remote_file.close().unwrap();
        remote_file.wait_close().unwrap();

        // TODO: change user and group of file to cached value beforehand...

        return ActionResult::Success;
    }

    fn copy_path(&self, connection: &mut ControlConnection, params: &ControlAction) -> ActionResult {
        let source_path = params.params.get_string_value("sourcePath");
        if source_path.is_none() {
            return ActionResult::InvalidParams;
        }
        let source_path = source_path.unwrap();

        let dest_path = params.params.get_string_value("destPath");
        if dest_path.is_none() {
            return ActionResult::InvalidParams;
        }
        let dest_path = dest_path.unwrap();

        let recursive = params.params.get_value_as_bool("recursive", false);
        let update = params.params.get_value_as_bool("update", false);

        let mut option_flags = String::new();
        if recursive {
            option_flags.push_str("-R");
        }
        if update {
            option_flags.push_str(" -u");
        }
        option_flags = option_flags.trim().to_string();

        let cp_command = format!(" cp {} {} {}", option_flags, source_path, dest_path);
        connection.conn.send_command(&cp_command);

        return ActionResult::Success;
    }
}

#[derive(Clone, Debug, PartialEq)]
enum ReplaceLineMatchType {
    Contains,
    StartsWith,
    EndsWith
}

struct ReplaceLineEntry {
    pub match_string:        String,
    pub replace_string:      String,
    pub report_failure:      bool,
    pub replaced:            bool,
    pub match_type:          ReplaceLineMatchType,
}

impl ReplaceLineEntry {
    pub fn new(match_string: &str, replace_string: &str, report_failure: bool, match_type: ReplaceLineMatchType) -> ReplaceLineEntry {
        ReplaceLineEntry { match_string: match_string.to_string(), replace_string: replace_string.to_string(),
             report_failure, replaced: false, match_type }
    }
}

fn extract_replace_line_entry_items_from_params_map(params: &Params, key: &str) -> Vec<ReplaceLineEntry> {
    let mut replace_line_entries = Vec::with_capacity(0);

    let param = params.get_raw_value(key);
    if let Some(ParamValue::Map(map)) = param {
        // cope with single items inline as map...
        if let Some(entry) = process_replace_line_entry(&map) {
            replace_line_entries.push(entry);
        }
    }
    else if let Some(ParamValue::Array(array)) = param {
        // cope with multiple items as an array
        for item in array {
            if let ParamValue::Map(map) = item {
                if let Some(entry) = process_replace_line_entry(&map) {
                    replace_line_entries.push(entry);
                }
            }
        }
    }

    return replace_line_entries;
}

fn process_replace_line_entry(entry: &BTreeMap<String, ParamValue>) -> Option<ReplaceLineEntry> {
    let match_string = entry.get("matchString");
    let mut match_string_val = String::new();
    if let Some(ParamValue::Str(string)) = match_string {
        match_string_val = string.clone();
    }
    let replace_string = entry.get("replaceString");
    let mut replace_string_val = String::new();
    if let Some(ParamValue::Str(string)) = replace_string {
        replace_string_val = string.clone();
    }
    
    let match_type = match entry.get("type") {
        Some(ParamValue::Str(str)) => {
            match str.as_str() {
                "contains" => ReplaceLineMatchType::Contains,
                "startsWith" => ReplaceLineMatchType::StartsWith,
                "endsWith" => ReplaceLineMatchType::EndsWith,
                _ => ReplaceLineMatchType::Contains
            }
        },
        _ => ReplaceLineMatchType::Contains
    };
    if !match_string_val.is_empty() && !replace_string_val.is_empty() {
        return Some(ReplaceLineEntry::new(&match_string_val, &replace_string_val, false, match_type));
    }

    return None;
}