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
use super::control_common::{ControlSession, ControlSessionParams, UserType};
use super::terminal_helpers_linux;

use std::path::Path;
use std::collections::BTreeMap;

use rpassword::read_password;

pub struct AProviderLinuxDebian {
    // params which give us some hints as to context of session, i.e. username - sudo vs root, etc.
    session_params: ControlSessionParams,
}

impl AProviderLinuxDebian {
    pub fn new(session_params: ControlSessionParams) -> AProviderLinuxDebian {
        AProviderLinuxDebian { session_params }
    }

    pub fn name() -> String {
        return "linux_debian".to_string();
    }

    fn post_process_command(&self, command: &str) -> String {
        let mut final_command = command.to_string();

        if self.session_params.user_type == UserType::Sudo {
            final_command.insert_str(0, "sudo ");
        }

        if self.session_params.hide_commands_from_history {
            final_command.insert(0, ' ');
        }

        return final_command;
    }
}

impl ActionProvider for AProviderLinuxDebian {
    fn name(&self) -> String {
        return "linux_debian".to_string();
    }

    fn generic_command(&self, connection: &mut ControlSession, action: &ControlAction) -> ActionResult {
        if !action.params.has_value("command") {
            return ActionResult::InvalidParams("The 'command' parameter was not specified.".to_string());
        }

        let command = action.params.get_string_value("command").unwrap();
        if !command.is_empty() {
            connection.conn.send_command(&self.post_process_command(&command));
        }

        // TODO: check if there's a 'errorIfStdErrOutputExists' param, and if so
        //       validate what the output of the command was...

        return ActionResult::Success;
    }

    fn add_user(&self, connection: &mut ControlSession, action: &ControlAction) -> ActionResult {
        // validate params
        if !action.params.has_value("username") {
            return ActionResult::InvalidParams("The 'username' parameter was not specified.".to_string());
        }
        if  !action.params.has_value("password") {
            return ActionResult::InvalidParams("The 'password' parameter was not specified.".to_string());
        }

        let mut useradd_command_options = String::new();

        let user = action.params.get_string_value("username").unwrap();
        let mut password = action.params.get_string_value("password").unwrap();
        if password == "$PROMPT" {
            eprintln!("Please enter password to set for user:");
            password = read_password().unwrap();
        }

        let create_home = action.params.get_value_as_bool("createHome", true);

        if create_home {
            useradd_command_options.push_str("-m ");
        }
        else {
            // do not create user's home group...
            useradd_command_options.push_str("-M ");
        }

        let shell = action.params.get_string_value_with_default("shell", "/bin/bash");
        useradd_command_options.push_str(&format!("-s {}", shell));

        let useradd_full_command = format!("useradd {} {}", useradd_command_options, user);

        connection.conn.send_command(&self.post_process_command(&useradd_full_command));

        // check response is nothing...
        if connection.conn.had_command_response() {
            return ActionResult::Failed("Unexpected response from useradd command.".to_string());
        }

        // double make sure we don't add command to history here, even though post_process_command() should do it
        // if required.
//        let change_password_command = format!(" echo -e \"{0}\n{0}\" | passwd {1}", password, user);
        let change_password_command = format!(" echo -e '{}:{}' | chpasswd", user, password);
        connection.conn.send_command(&self.post_process_command(&change_password_command));

        // now add user to any groups
        // see if there's just a single group...
        if action.params.has_value("group") {
            let usermod_command = format!("usermod -aG {} {}", action.params.get_string_value_with_default("group", ""), user);
            connection.conn.send_command(&self.post_process_command(&usermod_command));
        }
        else if action.params.has_value("groups") {
            // there's multiple
            let groups = action.params.get_values_as_vec_of_strings("groups");
            for group in groups {
                let usermod_command = format!("usermod -aG {} {}", group, user);
                connection.conn.send_command(&self.post_process_command(&usermod_command));
            }
        }

//        eprintln!("Added user okay.");

        return ActionResult::Success;
    }

    fn create_directory(&self, connection: &mut ControlSession, action: &ControlAction) -> ActionResult {
        // validate params
        if !action.params.has_value("path") {
            return ActionResult::InvalidParams("The 'path' parameter was not specified.".to_string());
        }

        let path_to_create = action.params.get_string_value("path").unwrap();

        // TODO: not sure about this... Maybe it should be called something else, maybe it should
        //       be the default?
        let multi_level = action.params.get_value_as_bool("multiLevel", false);
        let mnkdir_command;
        if !multi_level {
            mnkdir_command = format!("mkdir {}", path_to_create);
        }
        else {
            mnkdir_command = format!("mkdir -p {}", path_to_create);
        }
        connection.conn.send_command(&self.post_process_command(&mnkdir_command));

        if let Some(permissions) = action.params.get_string_or_int_value_as_string("permissions") {
            let chmod_command = format!("chmod {} {}", permissions, path_to_create);
            connection.conn.send_command(&self.post_process_command(&chmod_command));
        }

        if let Some(owner) = action.params.get_string_value("owner") {
            let chown_command = format!("chown {} {}", owner, path_to_create);
            connection.conn.send_command(&self.post_process_command(&chown_command));
        }

        if let Some(group) = action.params.get_string_value("group") {
            let chgrp_command = format!("chgrp {} {}", group, path_to_create);
            connection.conn.send_command(&self.post_process_command(&chgrp_command));
        }

        // TODO: check for 'groups' as well to handle setting multiple...

        return ActionResult::Success;
    }

    fn install_packages(&self, connection: &mut ControlSession, action: &ControlAction) -> ActionResult {
        // use apt-get, because the commands for that will apparently be much more stable, compared to apt
        // which might change as it's designed to be more user-facing...

        let packages_string;
        if let Some(package) = action.params.get_string_value("package") {
            // single package for convenience...
            packages_string = package;
        }
        else if action.params.has_value("packages") {
            let packages = action.params.get_values_as_vec_of_strings("packages");
            packages_string = packages.join(" ");
        }
        else {
            return ActionResult::InvalidParams("No 'package' string parameter or 'packages' string array parameter were specified.".to_string());
        }

        if packages_string.is_empty() {
            return ActionResult::InvalidParams("The 'packages' string list was empty.".to_string());
        }

        // with some providers (Vultr), apt-get runs automatically just after the instance first starts,
        // so we can't run apt-get manually, as the lock file is locked, so wait until apt-get has stopped running
        // by default... 
        let wait_for_apt_get_lockfile = action.params.get_value_as_bool("waitForPMToFinish", true);
        if wait_for_apt_get_lockfile {
            let mut try_count = 0;
            while try_count < 20 {
                connection.conn.send_command(&self.post_process_command("pidof apt-get"));

                if !connection.conn.had_command_response() {
                    // it's likely no longer running, so we can continue...
                    break;
                }

                // TODO: only print this once eventually, but might be useful like this for the moment...
                println!("Waiting for existing apt-get to finish before installing packages...");

                // sleep a bit to give things a chance...
                std::thread::sleep(std::time::Duration::from_secs(20));

                try_count += 1;
            }
        }

        // unattended-upgr

        // TODO: might be worth polling for locks on /var/lib/dpkg/lock-frontend ?

        // by default, update the list of packages, as with some providers,
        // this needs to be done first, otherwise packages can't be found...
        let update_packages = action.params.get_value_as_bool("update", true);
        if update_packages {
            let apt_get_command = "apt-get -y update".to_string();
            connection.conn.send_command(&self.post_process_command(&apt_get_command));
        }

        let apt_get_command = format!("apt-get -y install {}", packages_string);
        connection.conn.send_command(&self.post_process_command(&apt_get_command));

        if let Some(str) = connection.conn.get_previous_stderr_response() {
            println!("installPackages error: {}", str);
            return ActionResult::Failed(str.to_string());
        }

        return ActionResult::Success;
    }

    fn systemctrl(&self, connection: &mut ControlSession, action: &ControlAction) -> ActionResult {
        // validate params
        if !action.params.has_value("action") || !action.params.has_value("service") {
            return ActionResult::InvalidParams("".to_string());
        }

        let service = action.params.get_string_value("service").unwrap();
        let action = action.params.get_string_value("action").unwrap();

        let systemctrl_command = format!("systemctl {} {}", action, service);
        
        connection.conn.send_command(&self.post_process_command(&systemctrl_command));

        return ActionResult::Success;
    }

    fn firewall(&self, connection: &mut ControlSession, action: &ControlAction) -> ActionResult {
        let firewall_type = action.params.get_string_value_with_default("type", "ufw");
        if firewall_type != "ufw" {
            // only support this type for the moment...
            return ActionResult::InvalidParams("".to_string());
        }

        // incredibly basic for the moment...
        // in theory we should probably be more type-specific, and 'schema'd', but given there
        // are aliases for rules, it'd be quite complicated to handle that I think, so better
        // for the moment to allow freeform strings...
        let rules = action.params.get_values_as_vec_of_strings("rules");
        for rule in rules {
            let ufw_command = format!("ufw {}", rule);
            connection.conn.send_command(&self.post_process_command(&ufw_command));
        }

        if action.params.has_value("enabled") {
            let is_enabled = action.params.get_value_as_bool("enabled", true);
            let ufw_command = format!("ufw --force {}", if is_enabled { "enable" } else { "disable"});
            connection.conn.send_command(&self.post_process_command(&ufw_command));
        }

        return ActionResult::Success;
    }

    // TODO: this is pretty nasty and hacky, but works for all cases I want so far...
    fn edit_file(&self, connection: &mut ControlSession, action: &ControlAction) -> ActionResult {
        let filepath = action.params.get_string_value("filepath");
        if filepath.is_none() {
            return ActionResult::InvalidParams("The 'filepath' parameter was not specified.".to_string());
        }

        let replace_line_items = extract_edit_line_entry_items(&action.params, "replaceLine", &process_replace_line_entry);
        let insert_line_items = extract_edit_line_entry_items(&action.params, "insertLine", &process_insert_line_entry);
        if replace_line_items.is_empty() && insert_line_items.is_empty() {
            eprintln!("Error: editFile Control Action had no items to perform...");
            return ActionResult::InvalidParams("".to_string());
        }

        let filepath = filepath.unwrap();
        
        if action.params.get_value_as_bool("backup", false) {
            // TODO: something more robust than this...
            let mv_command = format!("cp {0} {0}.bak", filepath);
            connection.conn.send_command(&self.post_process_command(&mv_command));
        }

        // Note: the Stat returned by scp_recv() is currently a private field, so we can only access bits of it,
        //       so we need to do a full stat call remotely to get the actual info
        let stat_command = format!("stat {}", filepath);
        connection.conn.send_command(&self.post_process_command(&stat_command));

        let stat_response = connection.conn.get_previous_stdout_response().to_string();
        // get the details from the stat call...
        let stat_details = terminal_helpers_linux::extract_details_from_stat_output(&stat_response);

        // download the file
        let string_contents = connection.conn.get_text_file_contents(&filepath).unwrap();
        if string_contents.is_empty() {
            eprintln!("Error: remote file: {} has empty contents.", filepath);
            return ActionResult::Failed("".to_string());
        }
        let file_contents_lines = string_contents.lines();

        // brute force replacement - can optimise this or condense it, maybe both,
        // but just get it working for the moment...

        let item_matches_closure = |match_type : &FileEditMatchType, match_string: &str, line: &str| -> bool {
            if *match_type == FileEditMatchType::Contains {
                if line.contains(match_string) {
                    return true;
                }
            }
            else if *match_type == FileEditMatchType::Matches {
                if line == match_string {
                    return true;
                }
            }
            else if *match_type == FileEditMatchType::StartsWith {
                if line.starts_with(match_string) {
                    return true;
                }
            }
            else if *match_type == FileEditMatchType::EndsWith {
                if line.ends_with(match_string) {
                    return true;
                }
            }

            return false;
        };

        // this is disgusting, but can't be bothered with an enum...
        // TODO: also, this is going to be last-wins for any situation where multiple
        //       rules match a single line, but hopefully that won't happen for current use-cases...
        //       (clearly possible in theory though...)
        let mut insert_type = String::new();
        let mut insert_string = String::new();

        let mut new_file_contents_lines = Vec::new();
        for line in file_contents_lines {
            let mut have_replaced = false;
            
            insert_type.clear();
            insert_string.clear();

            // TODO: it's ambiguous what we should do when both an insert item and replace item
            //       might match a line, but let's just ignore handling that situation for the moment,
            //       and assume params will be set exclusively for each...

            for insert_item in &insert_line_items {
                if item_matches_closure(&insert_item.match_type, &insert_item.match_string, &line) {
                    insert_type = match insert_item.position_type {
                        InsertLinePositionType::Above => "A".to_string(),
                        InsertLinePositionType::Below => "B".to_string()
                    };
                    insert_string = insert_item.insert_string.clone();
                }
            }

            for replace_item in &replace_line_items {
                if item_matches_closure(&replace_item.match_type, &replace_item.match_string, &line) {
                    new_file_contents_lines.push(replace_item.replace_string.clone());
                    have_replaced = true;
                }
            }

            if !have_replaced {
                // as mentioned above, on the assumption there won't currently be replace AND insert for a single line...
                if insert_type == "A" {
                    new_file_contents_lines.push(insert_string.clone());
                }
                new_file_contents_lines.push(line.to_string());
                if insert_type == "B" {
                    new_file_contents_lines.push(insert_string.clone());
                }
            }
        }

        // convert back to single string for entire file, and make sure we append a newline on the end...
        let new_file_contents_string = new_file_contents_lines.join("\n") + "\n";

        let mode;
        if let Some(stat_d) = stat_details {
            mode = i32::from_str_radix(&stat_d.0, 8).unwrap();
        }
        else {
            mode = 0o644;
            eprintln!("Can't extract stat details from file. Using 644 as default permissions mode.");
        }
        
        let send_res = connection.conn.send_text_file_contents(&filepath, mode, &new_file_contents_string);
        if send_res.is_err() {
            return ActionResult::Failed("".to_string());
        }

        // TODO: change user and group of file to cached value from beforehand...

        return ActionResult::Success;
    }

    fn copy_path(&self, connection: &mut ControlSession, action: &ControlAction) -> ActionResult {
        let source_path = action.params.get_string_value("sourcePath");
        if source_path.is_none() {
            return ActionResult::InvalidParams("The 'sourcePath' parameter was not specified.".to_string());
        }
        let source_path = source_path.unwrap();

        let dest_path = action.params.get_string_value("destPath");
        if dest_path.is_none() {
            return ActionResult::InvalidParams("The 'destPath' parameter was not specified.".to_string());
        }
        let dest_path = dest_path.unwrap();

        let recursive = action.params.get_value_as_bool("recursive", false);
        let update = action.params.get_value_as_bool("update", false);

        let mut option_flags = String::new();
        if recursive {
            option_flags.push_str("-R");
        }
        if update {
            option_flags.push_str(" -u");
        }
        option_flags = option_flags.trim().to_string();

        let cp_command = format!("cp {} {} {}", option_flags, source_path, dest_path);
        connection.conn.send_command(&self.post_process_command(&cp_command));

        return ActionResult::Success;
    }

    fn download_file(&self, connection: &mut ControlSession, action: &ControlAction) -> ActionResult {
        let source_url = action.params.get_string_value("sourceURL");
        if source_url.is_none() {
            return ActionResult::InvalidParams("The 'sourceURL' parameter was not specified.".to_string());
        }
        let source_url = source_url.unwrap();

        let dest_path = action.params.get_string_value("destPath");
        if dest_path.is_none() {
            return ActionResult::InvalidParams("The 'destPath' parameter was not specified.".to_string());
        }
        let dest_path = dest_path.unwrap();

        // use wget (maybe curl backup?) for the moment
        let wget_command = format!("wget {} -O {}", source_url, dest_path);
        connection.conn.send_command(&self.post_process_command(&wget_command));

        if let Some(permissions) = action.params.get_string_or_int_value_as_string("permissions") {
            let chmod_command = format!("chmod {} {}", permissions, dest_path);
            connection.conn.send_command(&self.post_process_command(&chmod_command));
        }

        if let Some(owner) = action.params.get_string_value("owner") {
            let chown_command = format!("chown {} {}", owner, dest_path);
            connection.conn.send_command(&self.post_process_command(&chown_command));
        }

        if let Some(group) = action.params.get_string_value("group") {
            let chgrp_command = format!("chgrp {} {}", group, dest_path);
            connection.conn.send_command(&self.post_process_command(&chgrp_command));
        }

        return ActionResult::Success;
    }

    fn transmit_file(&self, connection: &mut ControlSession, action: &ControlAction) -> ActionResult {
        // local source path
        // TODO: not sure about this naming...
        let source_path = action.params.get_string_value("localSourcePath");
        if source_path.is_none() {
            return ActionResult::InvalidParams("The 'localSourcePath' parameter was not specified.".to_string());
        }
        let source_path = source_path.unwrap();

        // remote destination path
        let dest_path = action.params.get_string_value("remoteDestPath");
        if dest_path.is_none() {
            return ActionResult::InvalidParams("The 'remoteDestPath' parameter was not specified.".to_string());
        }
        let dest_path = dest_path.unwrap();

        let mut mode = 0o644;
        if let Some(permissions) = action.params.get_string_or_int_value_as_string("permissions") {
            mode = i32::from_str_radix(&permissions, 8).unwrap();
        }

        let send_res = connection.conn.send_file(&source_path, &dest_path, mode);
        if send_res.is_err() {
            return ActionResult::Failed("".to_string());
        }

        if let Some(owner) = action.params.get_string_value("owner") {
            let chown_command = format!("chown {} {}", owner, dest_path);
            connection.conn.send_command(&self.post_process_command(&chown_command));
        }

        if let Some(group) = action.params.get_string_value("group") {
            let chgrp_command = format!("chgrp {} {}", group, dest_path);
            connection.conn.send_command(&self.post_process_command(&chgrp_command));
        }

        return ActionResult::Success;
    }

    fn create_symlink(&self, connection: &mut ControlSession, action: &ControlAction) -> ActionResult {
        let target_path = action.params.get_string_value("targetPath");
        if target_path.is_none() {
            return ActionResult::InvalidParams("The 'targetPath' parameter was not specified.".to_string());
        }
        let target_path = target_path.unwrap();

        // link name / path
        let link_path = action.params.get_string_value("linkPath");
        if link_path.is_none() {
            return ActionResult::InvalidParams("The 'linkPath' parameter was not specified.".to_string());
        }
        let link_path = link_path.unwrap();

        let link_dir = Path::new(&link_path);
        // TODO: error handling - and this might be a directory?
        let link_name = link_dir.file_name().unwrap();
        let link_dir = link_dir.parent().unwrap().to_str().unwrap();

        // Note: currently this will have to be true I think, otherwise we'd be creating the link
        //       in the current working directory which is likely to be unhelpful...
        if link_path.contains('/') {     
            let cd_command = format!("cd {}", link_dir);
            connection.conn.send_command(&self.post_process_command(&cd_command));

            // check there was no error response
            if let Some(str) = connection.conn.get_previous_stderr_response() {
                eprintln!("create_symlink error: {}", str);
                return ActionResult::Failed(str.to_string());
            }
        }

        let ln_command = format!("ln -s {} {}", target_path, link_name.to_str().unwrap());
        connection.conn.send_command(&self.post_process_command(&ln_command));

        if let Some(str) = connection.conn.get_previous_stderr_response() {
            eprintln!("create_symlink error: {}", str);
            return ActionResult::Failed(str.to_string());
        }

        return ActionResult::Success;
    }

    fn set_time_zone(&self, connection: &mut ControlSession, action: &ControlAction) -> ActionResult {
        let time_zone = action.params.get_string_value("timeZone");
        if time_zone.is_none() {
            return ActionResult::InvalidParams("The 'timeZone' parameter was not specified.".to_string());
        }
        let time_zone = time_zone.unwrap();

        // "UTC", "Pacific/Auckland", "Europe/London"

        let timedatectl_command = format!("timedatectl {}", time_zone);
        connection.conn.send_command(&self.post_process_command(&timedatectl_command));

        if let Some(str) = connection.conn.get_previous_stderr_response() {
            eprintln!("set_time_zone error: {}", str);
            return ActionResult::Failed(str.to_string());
        }

        // TODO: also restart things like crond that might have been affected?

        return ActionResult::Success;
    }
}

// TODO: these two sets of enums/structs and functions have some duplication - see if we can reduce that...

#[derive(Clone, Debug, PartialEq)]
enum FileEditMatchType {
    Contains,
    Matches,
    StartsWith,
    EndsWith
}

struct ReplaceLineEntry {
    pub match_string:        String,
    pub replace_string:      String,
    pub report_failure:      bool,
    pub replaced:            bool,
    pub match_type:          FileEditMatchType,
}

impl ReplaceLineEntry {
    pub fn new(match_string: &str, replace_string: &str, report_failure: bool, match_type: FileEditMatchType) -> ReplaceLineEntry {
        ReplaceLineEntry { match_string: match_string.to_string(), replace_string: replace_string.to_string(),
             report_failure, replaced: false, match_type }
    }
}

fn extract_edit_line_entry_items<T>(params: &Params, key: &str, fun: &dyn Fn(&BTreeMap<String, ParamValue>) -> Option<T>) -> Vec<T> {
    let mut replace_line_entries = Vec::with_capacity(0);

    let param = params.get_raw_value(key);
    if let Some(ParamValue::Map(map)) = param {
        // cope with single items inline as map...
        if let Some(entry) = fun(&map) {
            replace_line_entries.push(entry);
        }
    }
    else if let Some(ParamValue::Array(array)) = param {
        // cope with multiple items as an array
        for item in array {
            if let ParamValue::Map(map) = item {
                if let Some(entry) = fun(&map) {
                    replace_line_entries.push(entry);
                }
            }
        }
    }

    return replace_line_entries;
}

fn get_replace_line_entry_match_type(entry: &BTreeMap<String, ParamValue>) -> FileEditMatchType {
    let match_type = match entry.get("matchType") {
        Some(ParamValue::Str(str)) => {
            match str.as_str() {
                "contains" => FileEditMatchType::Contains,
                "matches" => FileEditMatchType::Matches,
                "startsWith" => FileEditMatchType::StartsWith,
                "endsWith" => FileEditMatchType::EndsWith,
                _ => FileEditMatchType::Contains
            }
        },
        _ => FileEditMatchType::Contains
    };

    return match_type;
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
    
    let match_type = get_replace_line_entry_match_type(entry);

    if !match_string_val.is_empty() && !replace_string_val.is_empty() {
        return Some(ReplaceLineEntry::new(&match_string_val, &replace_string_val, false, match_type));
    }

    return None;
}

#[derive(Clone, Debug, PartialEq)]
enum InsertLinePositionType {
    Above,
    Below,
}

struct InsertLineEntry {
    pub position_type:       InsertLinePositionType,
    pub match_string:        String,
    pub insert_string:       String,
    pub report_failure:      bool,
    pub replaced:            bool,
    pub match_type:          FileEditMatchType,
}

impl InsertLineEntry {
    pub fn new(position_type: InsertLinePositionType, match_string: &str, insert_string: &str, report_failure: bool, match_type: FileEditMatchType) -> InsertLineEntry {
        InsertLineEntry { position_type, match_string: match_string.to_string(), insert_string: insert_string.to_string(),
             report_failure, replaced: false, match_type }
    }
}

fn process_insert_line_entry(entry: &BTreeMap<String, ParamValue>) -> Option<InsertLineEntry> {
    let match_string = entry.get("matchString");
    let mut match_string_val = String::new();
    if let Some(ParamValue::Str(string)) = match_string {
        match_string_val = string.clone();
    }
    let insert_string = entry.get("insertString");
    let mut insert_string_val = String::new();
    if let Some(ParamValue::Str(string)) = insert_string {
        insert_string_val = string.clone();
    }

    let position_type = match entry.get("position") {
        Some(ParamValue::Str(str)) => {
            match str.as_str() {
                "above" => InsertLinePositionType::Above,
                "below" => InsertLinePositionType::Below,
                _ => {
                    eprintln!("Warning: unrecognised 'position' value for insertLine entry item. Setting to 'below'.");
                    InsertLinePositionType::Below
                }
            }
        },
        _ => {
            eprintln!("Warning: undefined 'position' value for insertLine entry item. Setting to 'below'.");
            InsertLinePositionType::Below
        }
    };
    
    let match_type = get_replace_line_entry_match_type(entry);

    if !match_string_val.is_empty() && !insert_string_val.is_empty() {
        return Some(InsertLineEntry::new(position_type, &match_string_val, &insert_string_val, false, match_type));
    }

    return None;
}