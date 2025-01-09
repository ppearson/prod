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

use crate::control::terminal_helpers_linux;
use crate::params::ParamValue;

use super::common_actions_unix_edit_file;
use super::file_modifier_helpers::{modify_sshd_config_file_contents, ModifySshDConfigParams, SshDPermitRootLoginType};

use super::control_actions::{ActionProvider, ActionError, ControlAction};
use super::control_common::ControlSession;

pub fn generic_command(action_provider: &dyn ActionProvider, connection: &mut ControlSession, action: &ControlAction
) -> Result<(), ActionError> {
    let command = action.get_required_string_param("command")?;
    if !command.is_empty() {
        connection.conn.send_command(&action_provider.post_process_command(&command));
    }

    if action.params.get_value_as_bool("errorIfStdErrOutputExists").unwrap_or(false) {
        if let Some(strerr) = connection.conn.get_previous_stderr_response() {
            return Err(ActionError::FailedCommand(format!("genericCommand action failed due to unexpected stderr output: {}", strerr)));
        }
    }

    if action.params.get_value_as_bool("errorIfNone0ExitCode").unwrap_or(false) {
        if connection.conn.did_exit_with_error_code() {
            return Err(ActionError::FailedCommand("genericCommand action failed due to non-0 exit code.".to_string()));
        }
    }

    // TODO: support for specifying expected string output to look for (stderr/stdout)
    //       for success criteria...

    // TODO: support for specifying expected number of lines of output as well...

    Ok(())
}

pub fn create_directory(action_provider: &dyn ActionProvider, connection: &mut ControlSession, action: &ControlAction
) -> Result<(), ActionError> {
    let path_to_create = action.get_required_string_param("path")?;

    // TODO: not sure about this... Maybe it should be called something else, maybe it should
    //       be the default?
    let multi_level = action.params.get_value_as_bool("multiLevel").unwrap_or(false);
    let mkdir_command = if !multi_level {
        format!("mkdir {}", path_to_create)
    }
    else {
        format!("mkdir -p {}", path_to_create)
    };
    connection.conn.send_command(&action_provider.post_process_command(&mkdir_command));

    if connection.conn.did_exit_with_error_code() {
        return Err(ActionError::FailedCommand(connection.conn.return_failed_command_error_response_str(&mkdir_command,
            action)));
    }

    if let Some(permissions) = action.params.get_string_or_int_value_as_string("permissions") {
        let chmod_command = format!("chmod {} {}", permissions, path_to_create);
        connection.conn.send_command(&action_provider.post_process_command(&chmod_command));

        if connection.conn.did_exit_with_error_code() {
            return Err(ActionError::FailedCommand(connection.conn.return_failed_command_error_response_str(&chmod_command,
                action)));
        }
    }

    if let Some(owner) = action.params.get_string_value("owner") {
        let chown_command = format!("chown {} {}", owner, path_to_create);
        connection.conn.send_command(&action_provider.post_process_command(&chown_command));

        if connection.conn.did_exit_with_error_code() {
            return Err(ActionError::FailedCommand(connection.conn.return_failed_command_error_response_str(&chown_command,
                action)));
        }
    }

    if let Some(group) = action.params.get_string_value("group") {
        let chgrp_command = format!("chgrp {} {}", group, path_to_create);
        connection.conn.send_command(&action_provider.post_process_command(&chgrp_command));

        if connection.conn.did_exit_with_error_code() {
            return Err(ActionError::FailedCommand(connection.conn.return_failed_command_error_response_str(&chgrp_command,
                action)));
        }
    }

    // TODO: check for 'groups' as well to handle setting multiple...

    Ok(())
}

pub fn remove_directory(action_provider: &dyn ActionProvider, connection: &mut ControlSession, action: &ControlAction
) -> Result<(), ActionError> {
    let path_to_remove = action.get_required_string_param("path")?;

    let recursive = action.params.get_value_as_bool("recursive").unwrap_or(true);
    let rmdir_command = if !recursive {
        // TODO: Not really clear if this is worth it...
        format!("rmdir {}", path_to_remove)
    }
    else {
        format!("rm -rf {}", path_to_remove)
    };

    let ignore_failure = action.params.get_value_as_bool("ignoreFailure").unwrap_or(false);

    connection.conn.send_command(&action_provider.post_process_command(&rmdir_command));
    if !ignore_failure && connection.conn.did_exit_with_error_code() {
        return Err(ActionError::FailedCommand(connection.conn.return_failed_command_error_response_str(&rmdir_command,
            action)));
    }

    Ok(())
}

pub fn edit_file(action_provider: &dyn ActionProvider, connection: &mut ControlSession, action: &ControlAction
) -> Result<(), ActionError> {
    common_actions_unix_edit_file::edit_file(action_provider, connection, action)
}

pub fn copy_path(action_provider: &dyn ActionProvider, connection: &mut ControlSession, action: &ControlAction
) -> Result<(), ActionError> {
    let source_path = action.get_required_string_param("sourcePath")?;
    let dest_path = action.get_required_string_param("destPath")?;
   
    let recursive = action.params.get_value_as_bool("recursive").unwrap_or(false);
    let update = action.params.get_value_as_bool("update").unwrap_or(false);

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
        return Err(ActionError::FailedCommand(connection.conn.return_failed_command_error_response_str(&cp_command,
            action)));
    }

    Ok(())
}

pub fn remove_file(action_provider: &dyn ActionProvider, connection: &mut ControlSession, action: &ControlAction
) -> Result<(), ActionError> {
    let path = action.get_required_string_param("path")?;

    let rm_command = format!("rm {}", path);

    let ignore_failure = action.params.get_value_as_bool("ignoreFailure").unwrap_or(false);

    connection.conn.send_command(&action_provider.post_process_command(&rm_command));
    if !ignore_failure && connection.conn.did_exit_with_error_code() {
        return Err(ActionError::FailedCommand(connection.conn.return_failed_command_error_response_str(&rm_command,
            action)));
    }

    Ok(())
}

pub fn download_file(action_provider: &dyn ActionProvider, connection: &mut ControlSession, action: &ControlAction
) -> Result<(), ActionError> {
    let source_url = action.get_required_string_param("sourceURL")?;
    let dest_path = action.get_required_string_param("destPath")?;

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
        if !extract_dir.is_empty() {
            let test_cmd = format!("test -d {} && echo \"yep\"", extract_dir);
            connection.conn.send_command(&action_provider.post_process_command(&test_cmd));

            // check the output is "yep"
            if connection.conn.get_previous_stdout_response().is_empty() {
                // doesn't exist...
                return Err(ActionError::FailedOther(format!("The 'extractDir' parameter directory: '{}' does not exist.", extract_dir)));
            }

            // TODO: and check permissions?

            // now attempt to extract the file, by attempting to work out the filename
            if dest_path.ends_with(".zip") {
                // assume it's a .zip file...
                let zip_cmd = format!("unzip {} -d {}", dest_path, extract_dir);
                connection.conn.send_command(&action_provider.post_process_command(&zip_cmd));
            }
            else {
                // otherwise, assume it's some form of tar file...
                let tar_cmd = format!("tar -xf {} -C {}", dest_path, extract_dir);
                connection.conn.send_command(&action_provider.post_process_command(&tar_cmd));
            }
        }
    }

    Ok(())
}

pub fn transmit_file(action_provider: &dyn ActionProvider, connection: &mut ControlSession, action: &ControlAction
) -> Result<(), ActionError> {
    // local source path
    // TODO: not sure about this naming...
    let source_path = action.get_required_string_param("localSourcePath")?;
   
    // remote destination path
    let dest_path = action.get_required_string_param("remoteDestPath")?;

    let mut mode = 0o644;
    if let Some(permissions) = action.params.get_string_or_int_value_as_string("permissions") {
        mode = i32::from_str_radix(&permissions, 8).unwrap();
    }

    let send_res = connection.conn.send_file(&source_path, &dest_path, mode);
    if send_res.is_err() {
        return Err(ActionError::FailedOther("Failed to send file to host".to_string()));
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
        if !extract_dir.is_empty() {
            let test_cmd = format!("test -d {} && echo \"yep\"", extract_dir);
            connection.conn.send_command(&action_provider.post_process_command(&test_cmd));

            // check the output is "yep"
            if connection.conn.get_previous_stdout_response().is_empty() {
                // doesn't exist...
                return Err(ActionError::FailedOther(format!("The 'extractDir' parameter directory: '{}' does not exist.", extract_dir)));
            }

            // TODO: and check permissions?

            // now attempt to extract the file, by attempting to work out the filename
            if dest_path.ends_with(".zip") {
                // assume it's a .zip file...
                let zip_cmd = format!("unzip {} -d {}", dest_path, extract_dir);
                connection.conn.send_command(&action_provider.post_process_command(&zip_cmd));
            }
            else {
                // otherwise, assume it's some form of tar file...
                let tar_cmd = format!("tar -xf {} -C {}", dest_path, extract_dir);
                connection.conn.send_command(&action_provider.post_process_command(&tar_cmd));
            }

            // validate that extraction worked.
            // we can't *just* on stderr for this, as the terminal might not just print an error directly,
            // i.e., if say unzip is not installed, we don't get any stderr output, so we have
            // to rely on the exit code as well...
            if let Some(strerr) = connection.conn.get_previous_stderr_response() {
                return Err(ActionError::FailedCommand(format!("Failed to extract file: Err: {}", strerr)));
            }

            // also check exit code...
            if connection.conn.did_exit_with_error_code() {
                return Err(ActionError::FailedCommand("Failed to extract file, extraction command returned a non-0 exit code.".to_string()));
            }

            // otherwise, it's hopefully succeeded.
        }
    }

    Ok(())
}

pub fn receive_file(_action_provider: &dyn ActionProvider, connection: &mut ControlSession, action: &ControlAction
) -> Result<(), ActionError> {
    // remote source path
    // TODO: not sure about this naming...
    let source_path = action.get_required_string_param("remoteSourcePath")?;
 
    // local destination path
    let dest_path = action.get_required_string_param("localDestPath")?;

    let send_res = connection.conn.receive_file(&source_path, &dest_path);
    if send_res.is_err() {
        return Err(ActionError::FailedOther("Failed to receive file from host".to_string()));
    }

    Ok(())
}

pub fn create_symlink(action_provider: &dyn ActionProvider, connection: &mut ControlSession, action: &ControlAction
) -> Result<(), ActionError> {
    let target_path = action.get_required_string_param("targetPath")?;
  
    // link path
    let link_path = action.get_required_string_param("linkPath")?;
    let ln_command = format!("ln -s {} {}", target_path, link_path);
    connection.conn.send_command(&action_provider.post_process_command(&ln_command));

    if connection.conn.did_exit_with_error_code() {
        return Err(ActionError::FailedCommand(connection.conn.return_failed_command_error_response_str(&ln_command,
            action)));
    }

    Ok(())
}

pub fn create_file(action_provider: &dyn ActionProvider, connection: &mut ControlSession, action: &ControlAction
) -> Result<(), ActionError> {
    let path_to_create = action.get_required_string_param("path")?;

    // TODO: maybe add support for creating any subdirs if required?
    
    // see if there's any content we need
    if let Some(content) = action.params.get_string_value("content") {
        // send the content as a file to write
        let send_res = connection.conn.send_text_file_contents(&path_to_create, 0o644, &content);
        if send_res.is_err() {
            return Err(ActionError::FailedOther("Failed to send text file contents to create file.".to_string()));
        }
    }
    else {
        // create an empty file, as there was no content param specified.
        let touch_command = format!("touch {}", path_to_create);
        connection.conn.send_command(&action_provider.post_process_command(&touch_command));
        if let Some(strerr) = connection.conn.get_previous_stderr_response() {
            return Err(ActionError::FailedOther(format!("Failed to create file: Err: {}", strerr)));
        }
    }

    // TODO: maybe move this somewhere more common, so it can be shared more?
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

    Ok(())
}

// Note: rather than using the exiting EditFile functionality (which needs improvement), for the moment this is using
//       bespoke other code to make the config file changes, so as to hopefully be a bit more robust to variations
//       in things like whitespace...
pub fn configure_ssh(action_provider: &dyn ActionProvider, connection: &mut ControlSession, action: &ControlAction
) -> Result<(), ActionError> {

    let mut modify_sshd_config = ModifySshDConfigParams::new();

    // work out what we're doing first from the params...
    // Note: this param interpretation is done a bit differently for this action method, in order
    //       to also allow "yes"/"no" values to match sshd_config (not sure how good an idea this is, but)...
    if let Some(password_authentication) = action.params.get_value_as_bool("passwordAuthentication") {
        modify_sshd_config.password_authentication = Some(password_authentication);
    }

    if let Some(permit_empty_passwords) = action.params.get_value_as_bool("permitEmptyPasswords") {
        modify_sshd_config.permit_empty_passwords = Some(permit_empty_passwords);
    }

    // this one's a bit awkward, as it can be a third value, as well as true or false,
    // so rather than getting it as a bool, get it as a raw string...
    if let Some(permit_root_login) = action.params.get_raw_value("permitRootLogin") {
        let set_val = match permit_root_login {
            ParamValue::Bool(true) => Some(SshDPermitRootLoginType::Yes),
            ParamValue::Bool(false) => Some(SshDPermitRootLoginType::No),
            ParamValue::Str(str_val) => {
                match str_val.as_str() {
                    "prohibit-password" => Some(SshDPermitRootLoginType::ProhibitPassword),
                    _ => None,
                }
            },
            _ => None,
        };
        if set_val.is_none() {
            return Err(ActionError::InvalidParams(
                format!("Unrecognised value '{}' for 'permitRootLogin' param of configureSSH action.", permit_root_login)));
        }
        modify_sshd_config.permit_root_login = set_val;
    }

    if let Some(port_num) = action.params.get_value_as_int("port") {
        if port_num > 0 && port_num <= u16::MAX.into() {
            modify_sshd_config.port = Some(port_num as u16);
        }
        else {
            return Err(ActionError::InvalidParams("Couldn't parse 'port' param correctly for configureSSH action.".to_string()));
        }
    }

    if let Some(pub_key_authentication) = action.params.get_value_as_bool("pubKeyAuthentication") {
        modify_sshd_config.pub_key_authentication = Some(pub_key_authentication);
    }

    // if nothing was actually set, it'd be a no-op, so error...
    if !modify_sshd_config.any_set() {
        return Err(ActionError::InvalidParams("No valid parameters were set for this configureSSH control action, so it will not do anything.".to_string()));
    }

    // Note: this is correct for most Linux distros (and FreeBSD I think), however I'm not sure it is for other things
    //       like MacOS, so if we ever do support more action providers than Linux ones, we might need to conditionally
    //       change this...
    const REMOTE_CONF_FILEPATH: &str = "/etc/ssh/sshd_config";
 
    // Note: the Stat returned by scp_recv() is currently a private field, so we can only access bits of it,
    //       so we need to do a full stat call remotely to get the actual info
    let stat_command = format!("stat {}", REMOTE_CONF_FILEPATH);
    connection.conn.send_command(&action_provider.post_process_command(&stat_command));
    if let Some(strerr) = connection.conn.get_previous_stderr_response() {
        return Err(ActionError::FailedOther(format!("Error accessing remote file path: {}", strerr)));
    }

    // make a backup if required
    if action.params.get_value_as_bool("backup").unwrap_or(false) {
        // TODO: something more robust than this...
        let mv_command = format!("cp {0} {0}.bak", REMOTE_CONF_FILEPATH);
        connection.conn.send_command(&action_provider.post_process_command(&mv_command));
        if let Some(strerr) = connection.conn.get_previous_stderr_response() {
            return Err(ActionError::FailedOther(format!("Error making backup copy of remote file path: {}", strerr)));
        }
    }

    let stat_response = connection.conn.get_previous_stdout_response().to_string();
    // get the details from the stat call...
    let stat_details = terminal_helpers_linux::extract_details_from_stat_output(&stat_response);

    // download the file
    let string_contents = connection.conn.get_text_file_contents(REMOTE_CONF_FILEPATH).unwrap();
    if string_contents.is_empty() {
        eprintln!("Error: remote file: {} has empty contents.", REMOTE_CONF_FILEPATH);
        return Err(ActionError::FailedOther("".to_string()));
    }

    let modified_file_contents = modify_sshd_config_file_contents(&string_contents, &modify_sshd_config);
    // this can't currently fail, but...
    let modified_file_contents = modified_file_contents.unwrap();

    let mode;
    if let Some(stat_d) = stat_details {
        mode = i32::from_str_radix(&stat_d.0, 8).unwrap();
    }
    else {
        mode = 0o644;
        eprintln!("Can't extract stat details from file. Using 644 as default permissions mode.");
    }
    
    let send_res = connection.conn.send_text_file_contents(REMOTE_CONF_FILEPATH, mode, &modified_file_contents);
    if let Err(err) = send_res {
        return Err(ActionError::FailedOther(format!("Failed to send new sshd_conf file contents back to host: {}", err)));
    }

    // TODO: change user and group of file to cached value from beforehand...

    // assume the edit operation didn't obviously fail, so likely succeeded...

    // TODO: test with sshd -T  ?

    // now restart the sshd service, unless we were asked not to (default is true)
    let restart_sshd_service = action.params.get_value_as_bool("restartService").unwrap_or(true);
    if restart_sshd_service {
        let systemctrl_restart_command = "systemctl restart sshd";
        connection.conn.send_command(&action_provider.post_process_command(systemctrl_restart_command));

        if connection.conn.did_exit_with_error_code() {
            return Err(ActionError::FailedCommand(connection.conn.return_failed_command_error_response_str(systemctrl_restart_command,
                action)));
        }
    }

    Ok(())
}