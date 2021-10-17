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

#![allow(dead_code)]

use ssh2::Session;
use std::io::prelude::*;
use std::net::TcpStream;

extern crate rpassword;
use rpassword::read_password;

use crate::control::control_actions::ControlActionType;
use crate::control::control_common::ControlConnection;

use super::control_actions::{ControlActions, ActionProvider};

use super::action_provider_linux_debian;

pub struct ControlManager {
    registered_action_providers: Vec<Box<dyn ActionProvider> >
}

#[derive(Clone, Debug)]
pub enum CommandResult {
    ErrorCantConnect(String),
    ErrorAuthenticationIssue(String),
    Failed(String),
    CommandRunOkay(String),
}

impl ControlManager {
    pub fn new() -> ControlManager {
        let mut manager = ControlManager { registered_action_providers: Vec::new() };

        let new_provider = action_provider_linux_debian::AProviderLinuxDebian::new();
        manager.registered_action_providers.push(Box::new(new_provider));

        return manager;
    }

    fn find_provider(&self, provider: &str) -> Option<&dyn ActionProvider> {
        for prov in &self.registered_action_providers {
            if prov.name() == provider {
                return Some(prov.as_ref());
            }
        }

        return None;
    }

    pub fn run_command(&self, host: &str, command: &str) -> CommandResult {
        println!("Connecting to host: {}...", host);

        let host_target = format!("{}:22", host);
        let tcp_connection = TcpStream::connect(&host_target);
        if tcp_connection.is_err() {
            return CommandResult::ErrorCantConnect("".to_string());
        }
        let tcp_connection = tcp_connection.unwrap();
        let mut sess = Session::new().unwrap();

        println!("Enter password:");
        let password = read_password().unwrap();

        sess.set_tcp_stream(tcp_connection);
        sess.handshake().unwrap();
        let auth_res = sess.userauth_password("peter", &password);
        if auth_res.is_err() {
            return CommandResult::ErrorAuthenticationIssue(format!("{:?}", auth_res.err()));
        }

        let mut channel = sess.channel_session().unwrap();
        channel.exec(&command).unwrap();
        let mut result_string = String::new();
        channel.read_to_string(&mut result_string).unwrap();

        return CommandResult::CommandRunOkay(result_string);
    }

    pub fn perform_actions(&self, actions: &ControlActions) {
        let provider = self.find_provider(&actions.provider);

        if provider.is_none() {
            eprintln!("Error: Can't find provider: '{}'.", actions.provider);
            return;
        }

        let provider = provider.unwrap();

        let mut hostname = String::new();
        if actions.host.is_empty() || actions.host == "$PROMPT" {
            eprintln!("Please enter hostname to connect to:");
            std::io::stdin().read_line(&mut hostname).expect("Error reading hostname from std input");
            hostname = hostname.trim().to_string();
        }
        else {
            hostname = actions.host.clone();
        }

        // connect to host
        let host_target = format!("{}:22", hostname);
        let tcp_connection = TcpStream::connect(&host_target);
        if tcp_connection.is_err() {
            eprintln!("Error: Can't connect to host: '{}'.", host_target);
            return;
        }
        let tcp_connection = tcp_connection.unwrap();
        let mut sess = Session::new().unwrap();

        let mut username = String::new();
        if actions.user.is_empty() || actions.user == "$PROMPT" {
            eprintln!("Please enter username to authenticate with:");
            std::io::stdin().read_line(&mut username).expect("Error reading username from std input");
            username = username.trim().to_string();
        }
        else {
            username = actions.user.clone();
        }

        println!("Enter password:");
        let password = read_password().unwrap();

        sess.set_tcp_stream(tcp_connection);
        sess.handshake().unwrap();
        let auth_res = sess.userauth_password(&username, &password);
        if auth_res.is_err() {
            eprintln!("Error: Authentication failure with user: {}...", username);
            return;
        }

        let mut connection = ControlConnection::new(sess);

        eprintln!("Running actions...");

        for action in &actions.actions {
            // TODO: much better (automatic - based off lookup) despatch than this...
            if action.action == ControlActionType::AddUser {
                provider.add_user(&mut connection, &action);
            }
        }
    }
}

