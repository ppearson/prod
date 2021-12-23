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

use ureq;
//use ureq::Error;
use serde::{Deserialize, Serialize};

//use std::collections::BTreeSet;

use crate::provision::provision_provider::{ProvisionProvider};
//use crate::provision::provision_common::{ActionResultValues, ProvisionActionResult, ProvisionActionType, ProvisionResponseWaitType};
use crate::provision::provision_manager::{ListType};
//use crate::provision::provision_params::{ProvisionParams};

use crate::column_list_printer::{ColumnListPrinter, Alignment};

#[derive(Serialize, Deserialize)]
struct SizeResultItem {
    slug: String,
    // Note: these are in practice null, so no point looking for them...
    // description: String,
    // cpu_description: String,
    // storage_description: String,

    available: bool,

    price_monthly: f32,
    price_hourly: f32,

    vcpus: u32,

    memory: u32,
    disk: u32,
    transfer: f32,    
}

#[derive(Serialize, Deserialize)]
struct SizeListResults {
    sizes: Vec<SizeResultItem>
}

#[derive(Serialize, Deserialize)]
struct ImageResultItem {
    id: u64,
    name: String,
    distribution: String,
    full_name: String,
    slug: String,
    public: bool,
    min_disk_size: u32,

    #[serde(rename = "type")]
    ttype: String,

    size_gigabytes: f32,
    status: String,
}

#[derive(Serialize, Deserialize)]
struct ImageListResults {
    images: Vec<ImageResultItem>
}

#[derive(Serialize, Deserialize)]
struct RegionResultItem {
    name: String,
    slug: String,
    available: bool,
}

#[derive(Serialize, Deserialize)]
struct RegionListResults {
    regions: Vec<RegionResultItem>
}

pub struct ProviderBinaryLane {
    binary_lane_api_token: String,
}

impl ProviderBinaryLane {
    pub fn new() -> ProviderBinaryLane {
        ProviderBinaryLane { binary_lane_api_token: String::new() }
    }
}

impl ProvisionProvider for ProviderBinaryLane {
    fn name(&self) -> String {
        return "binary_lane".to_string();
    }

    fn supports_interactive(&self) -> bool {
        return true;
    }

    fn prompt_interactive(&self) -> Vec<(String, String)> {
        let mut items = Vec::new();
        items.push(("API_TOKEN".to_string(), "API Token to use Binary Lane API".to_string()));
        return items;
    }

    fn configure_interactive(&mut self) -> bool {
        return false;
    }

    fn configure(&mut self) -> bool {
        let binary_lane_api_token_env = std::env::var("PROD_BINARY_LANE_API_TOKEN");
        match binary_lane_api_token_env {
            Err(_e) => {
                // silently fail...
//                eprintln!("Error: $PROD_BINARY_LANE_API_TOKEN not set correctly.");
                return false;
            }
            Ok(v) => {
                self.binary_lane_api_token = v.trim().to_string();
                return true;
            }
        }
    }

    fn is_configured(&self) -> bool {
        return !self.binary_lane_api_token.is_empty();
    }

    // actual commands

    fn list_available(&self, list_type: ListType) -> bool {
        let url = match list_type {
            ListType::Plans => "sizes",
            ListType::Regions => "regions",
            ListType::OSs => "images",
            _ => {
                return false;
            }
        };

        if list_type == ListType::OSs {
            // Note: to list images, the request needs to be authenticated with a bearer
            if self.binary_lane_api_token.is_empty() {
                eprintln!("Binary Lane requires an API token to be used for OS/image list API requests. Please set $PROD_BINARY_LANE_API_TOKEN.");
                return false;
            }
        }

        let mut request = ureq::get(&format!("https://api.binarylane.com.au/v2/{}", url));
        if list_type == ListType::OSs {
            request = request.set("Authorization", &format!("Bearer {}", self.binary_lane_api_token));
        }

        let resp = request.call();        
        if resp.is_err() {
            eprintln!("Error querying api.binarylane.com.au for list request: {:?}", resp.err());
            return false;
        }

        let resp_string = resp.unwrap().into_string().unwrap();

        if list_type == ListType::Regions {
            let results: RegionListResults = serde_json::from_str(&resp_string).unwrap();

            println!("{} regions:", results.regions.len());

            let mut clp = ColumnListPrinter::new(3)
                .add_titles(["id", "name", "available"]);

            for region in &results.regions {
                clp.add_row_strings(&[&region.slug, &region.name, if region.available {"true"} else {"false"}]);
            }

            print!("{}", clp);
        }
        else if list_type == ListType::Plans {
            let results: SizeListResults = serde_json::from_str(&resp_string).unwrap();

            println!("{} plans:", results.sizes.len());

            let mut clp = ColumnListPrinter::new(6)
                .set_alignment_multiple(&vec![2usize, 3, 4, 5], Alignment::Right)
                .add_titles(["id", "cpus", "memory", "disk", "transfer", "price"]);

            for size in &results.sizes {
                clp.add_row_strings(&[&size.slug, &format!("{}", size.vcpus), &format!("{} MB", size.memory),
                                         &format!("{} GB", size.disk), &format!("{:.2} TB", size.transfer), &format!("${:.2}", size.price_monthly)]);
            }

            print!("{}", clp);
        }
        else if list_type == ListType::OSs {            
            let results: ImageListResults = serde_json::from_str(&resp_string).unwrap();

            println!("{} OS images:", results.images.len());

            let mut clp = ColumnListPrinter::new(5);

            for image in &results.images {
                clp.add_row_strings(&[&format!("{}", image.id), &image.distribution, &image.ttype, &image.full_name, &image.status]);
            }

            print!("{}", clp);
        }
        else {
            println!("{}", resp_string);
        }
        
        return true;
    }
}