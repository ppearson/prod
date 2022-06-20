/*
 Prod
 Copyright 2021-2022 Peter Pearson.
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

use super::common_actions_unix_edit_file;

use super::control_actions::{ActionProvider, ActionResult, ControlAction};
use super::control_common::{ControlSession};

use std::path::Path;

pub fn generic_command(action_provider: &dyn ActionProvider, connection: &mut ControlSession, action: &ControlAction) -> ActionResult {
    if !action.params.has_value("command") {
        return ActionResult::InvalidParams("The 'command' parameter was not specified.".to_string());
    }

    let command = action.params.get_string_value("command").unwrap();
    if !command.is_empty() {
        connection.conn.send_command(&action_provider.post_process_command(&command));
    }

    // TODO: check if there's a 'errorIfStdErrOutputExists' param, and if so
    //       validate what the output of the command was...

    return ActionResult::Success;
}

pub fn create_directory(action_provider: &dyn ActionProvider, connection: &mut ControlSession, action: &ControlAction) -> ActionResult {
    // validate params
    if !action.params.has_value("path") {
        return ActionResult::InvalidParams("The 'path' parameter was not specified.".to_string());
    }

    let path_to_create = action.params.get_string_value("path").unwrap();

    // TODO: not sure about this... Maybe it should be called something else, maybe it should
    //       be the default?
    let multi_level = action.params.get_value_as_bool("multiLevel", false);
    let mkdir_command;
    if !multi_level {
        mkdir_command = format!("mkdir {}", path_to_create);
    }
    else {
        mkdir_command = format!("mkdir -p {}", path_to_create);
    }
    connection.conn.send_command(&action_provider.post_process_command(&mkdir_command));
    if let Some(strerr) = connection.conn.get_previous_stderr_response() {
        return ActionResult::Failed(format!("Failed to create directory: Err: {}", strerr));
    }

    if let Some(permissions) = action.params.get_string_or_int_value_as_string("permissions") {
        let chmod_command = format!("chmod {} {}", permissions, path_to_create);
        connection.conn.send_command(&action_provider.post_process_command(&chmod_command));
    }

    if let Some(owner) = action.params.get_string_value("owner") {
        let chown_command = format!("chown {} {}", owner, path_to_create);
        connection.conn.send_command(&action_provider.post_process_command(&chown_command));
    }

    if let Some(group) = action.params.get_string_value("group") {
        let chgrp_command = format!("chgrp {} {}", group, path_to_create);
        connection.conn.send_command(&action_provider.post_process_command(&chgrp_command));
    }

    // TODO: check for 'groups' as well to handle setting multiple...

    return ActionResult::Success;
}

pub fn remove_directory(action_provider: &dyn ActionProvider, connection: &mut ControlSession, action: &ControlAction) -> ActionResult {
    // validate params
    if !action.params.has_value("path") {
        return ActionResult::InvalidParams("The 'path' parameter was not specified.".to_string());
    }

    let path_to_remove = action.params.get_string_value("path").unwrap();

    let recursive = action.params.get_value_as_bool("recursive", true);
    let rmdir_command;
    if !recursive {
        // TODO: Not really clear if this is worth it...
        rmdir_command = format!("rmdir {}", path_to_remove);
    }
    else {
        rmdir_command = format!("rm -rf {}", path_to_remove);
    }

    let ignore_failure = action.params.get_value_as_bool("ignoreFailure", false);

    connection.conn.send_command(&action_provider.post_process_command(&rmdir_command));
    if let Some(strerr) = connection.conn.get_previous_stderr_response() {
        if !ignore_failure {
            return ActionResult::Failed(format!("Failed to remove directory: Err: {}", strerr));
        }
    }

    return ActionResult::Success;
}

pub fn edit_file(action_provider: &dyn ActionProvider, connection: &mut ControlSession, action: &ControlAction) -> ActionResult {
    return common_actions_unix_edit_file::edit_file(action_provider, connection, action);
}

pub fn copy_path(action_provider: &dyn ActionProvider, connection: &mut ControlSession, action: &ControlAction) -> ActionResult {
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
    connection.conn.send_command(&action_provider.post_process_command(&cp_command));

    if connection.conn.did_exit_with_error_code() {
        return ActionResult::Failed(format!("Unexpected response from '{}' command: {}", cp_command,
                connection.conn.get_previous_stderr_response().unwrap_or("")));
    }

    return ActionResult::Success;
}

pub fn remove_file(action_provider: &dyn ActionProvider, connection: &mut ControlSession, action: &ControlAction) -> ActionResult {
    let path = action.params.get_string_value("path");
    if path.is_none() {
        return ActionResult::InvalidParams("The 'path' parameter was not specified.".to_string());
    }
    let path = path.unwrap();

    let rm_command = format!("rm {}", path);

    let ignore_failure = action.params.get_value_as_bool("ignoreFailure", false);

    connection.conn.send_command(&action_provider.post_process_command(&rm_command));
    if let Some(strerr) = connection.conn.get_previous_stderr_response() {
        if !ignore_failure {
            return ActionResult::Failed(format!("Failed to remove file: Err: {}", strerr));
        }
    }

    return ActionResult::Success;
}

pub fn download_file(action_provider: &dyn ActionProvider, connection: &mut ControlSession, action: &ControlAction) -> ActionResult {
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
    connection.conn.send_command(&action_provider.post_process_command(&wget_command));

    if let Some(permissions) = action.params.get_string_or_int_value_as_string("permissions") {
        let chmod_command = format!("chmod {} {}", permissions, dest_path);
        connection.conn.send_command(&action_provider.post_process_command(&chmod_command));
    }

    if let Some(owner) = action.params.get_string_value("owner") {
        let chown_command = format!("chown {} {}", owner, dest_path);
        connection.conn.send_command(&action_provider.post_process_command(&chown_command));
    }

    if let Some(group) = action.params.get_string_value("group") {
        let chgrp_command = format!("chgrp {} {}", group, dest_path);
        connection.conn.send_command(&action_provider.post_process_command(&chgrp_command));
    }

    // see if we should also extract it
    if let Some(extract_dir) = action.params.get_string_value("extractDir") {
        // check this directory actually exists...
        if !extract_dir.is_empty()
        {
            let test_cmd = format!("test -d {} && echo \"yep\"", extract_dir);
            connection.conn.send_command(&action_provider.post_process_command(&test_cmd));

            // check the output is "yep"
            if connection.conn.get_previous_stdout_response().is_empty() {
                // doesn't exist...
                return ActionResult::Failed(format!("The 'extractDir' parameter directory: '{}' does not exist.", extract_dir));
            }

            // TODO: and check permissions?

            // now attempt to extract the file
            let tar_cmd = format!("tar -xf {} -C {}", dest_path, extract_dir);
            connection.conn.send_command(&action_provider.post_process_command(&tar_cmd));
        }
    }

    return ActionResult::Success;
}

pub fn transmit_file(action_provider: &dyn ActionProvider, connection: &mut ControlSession, action: &ControlAction) -> ActionResult {
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
        connection.conn.send_command(&action_provider.post_process_command(&chown_command));
    }

    if let Some(group) = action.params.get_string_value("group") {
        let chgrp_command = format!("chgrp {} {}", group, dest_path);
        connection.conn.send_command(&action_provider.post_process_command(&chgrp_command));
    }

    // see if we should also extract it
    if let Some(extract_dir) = action.params.get_string_value("extractDir") {
        // check this directory actually exists...
        if !extract_dir.is_empty()
        {
            let test_cmd = format!("test -d {} && echo \"yep\"", extract_dir);
            connection.conn.send_command(&action_provider.post_process_command(&test_cmd));

            // check the output is "yep"
            if connection.conn.get_previous_stdout_response().is_empty() {
                // doesn't exist...
                return ActionResult::Failed(format!("The 'extractDir' parameter directory: '{}' does not exist.", extract_dir));
            }

            // TODO: and check permissions?

            // now attempt to extract the file
            let tar_cmd = format!("tar -xf {} -C {}", dest_path, extract_dir);
            connection.conn.send_command(&action_provider.post_process_command(&tar_cmd));
        }
    }

    return ActionResult::Success;
}

pub fn receive_file(_action_provider: &dyn ActionProvider, connection: &mut ControlSession, action: &ControlAction) -> ActionResult {
    // remote source path
    // TODO: not sure about this naming...
    let source_path = action.params.get_string_value("remoteSourcePath");
    if source_path.is_none() {
        return ActionResult::InvalidParams("The 'remoteSourcePath' parameter was not specified.".to_string());
    }
    let source_path = source_path.unwrap();

    // local destination path
    let dest_path = action.params.get_string_value("localDestPath");
    if dest_path.is_none() {
        return ActionResult::InvalidParams("The 'localDestPath' parameter was not specified.".to_string());
    }
    let dest_path = dest_path.unwrap();

    let send_res = connection.conn.receive_file(&source_path, &dest_path);
    if send_res.is_err() {
        return ActionResult::Failed("".to_string());
    }

    return ActionResult::Success;
}

pub fn create_symlink(action_provider: &dyn ActionProvider, connection: &mut ControlSession, action: &ControlAction) -> ActionResult {
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
        connection.conn.send_command(&action_provider.post_process_command(&cd_command));

        // check there was no error response
        if let Some(str) = connection.conn.get_previous_stderr_response() {
            eprintln!("create_symlink error: {}", str);
            return ActionResult::Failed(str.to_string());
        }
    }

    let ln_command = format!("ln -s {} {}", target_path, link_name.to_str().unwrap());
    connection.conn.send_command(&action_provider.post_process_command(&ln_command));

    if let Some(str) = connection.conn.get_previous_stderr_response() {
        eprintln!("create_symlink error: {}", str);
        return ActionResult::Failed(str.to_string());
    }

    return ActionResult::Success;
}
