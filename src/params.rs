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

use std::collections::BTreeMap;
use std::fmt;
use std::convert::From;

use yaml_rust::Yaml;

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub enum ParamValue {
    NotSet,
    Unknown,
    Bool(bool),
    Int(i32),
    Str(String),
    Array(Vec<ParamValue>),
    Map(BTreeMap<String, ParamValue>)
}

impl fmt::Display for ParamValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self {
            ParamValue::NotSet => write!(f, "NotSet"),
            ParamValue::Unknown => write!(f, "Unknown"),
            ParamValue::Bool(b) => write!(f, "{}", if *b == true {"true"} else {"false"}),
            ParamValue::Int(i) => write!(f, "{}", i),
            ParamValue::Str(s) => write!(f, "'{}'", s),
            ParamValue::Array(arr) => {
                write!(f, "array [{}] = {{", arr.len())?;
                for it in arr {
                    write!(f, " {},", it)?;
                }
                write!(f, " }}")
            },
            ParamValue::Map(map) => {
                // TODO:
                write!(f, "{{")?;
                for (key, val) in map {
                    write!(f, " {}: {}, ", key, val)?;
                }
                write!(f, "}}")
            }
        }
    }
}

impl From<Yaml> for ParamValue {
    fn from(item: Yaml) -> Self {
        match item {
            Yaml::Boolean(v) => ParamValue::Bool(v),
            Yaml::Integer(v) => ParamValue::Int(v as i32),
            Yaml::String(v) => ParamValue::Str(v),
            Yaml::Array(v) => {
                let mut new_vec = Vec::with_capacity(v.len());
                for it in v {
                    new_vec.push(ParamValue::from(it));
                }
                ParamValue::Array(new_vec)
            },
            Yaml::Hash(v) => {
                let mut new_map = BTreeMap::new();

                for (key, val) in v {
                    if let Yaml::String(key_string) = key {
                        let val_param = ParamValue::from(val);
                        new_map.insert(key_string, val_param);
                    }
                    else {
                        // TOOD: error handling...
                    }
                }

                ParamValue::Map(new_map)
            },
            _ => ParamValue::Unknown
        }
    }
}

#[derive(Clone, Debug)]
pub struct Params {
    pub values:     BTreeMap<String, ParamValue>,
}

impl fmt::Display for Params {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Params: ({}) {{", self.values.len())?;
        for (key, val) in self.values.clone() {
            writeln!(f, " {}: {}", key, val)?;
        }
        write!(f, "}}")
    }
}

impl Params {
    pub fn new() -> Params {
        Params { values: BTreeMap::new() }
    }

    pub fn has_value(&self, key: &str) -> bool {
        self.values.contains_key(key)
    }

    // TODO: YAML is really annoying with numbers, so this is rather hacky...
    //       Would be better to return an enum or something of the actual type...
    pub fn get_string_or_int_value_as_string(&self, key: &str) -> Option<String> {
        let res = self.values.get(key);
        match res {
            Some(ParamValue::Str(str_val)) => {
                return Some(str_val.to_string());
            },
            Some(ParamValue::Int(int_val)) => {
                return Some(format!("{}", int_val));
            }
            _ => {}
        };

        None
    }

    pub fn get_string_value(&self, key: &str) -> Option<String> {
        let res = self.values.get(key);
        if let Some(ParamValue::Str(str_val)) = res {
            return Some(str_val.to_string());
        }

        None
    }

    pub fn get_string_value_with_default(&self, key: &str, default: &str) -> String {
        let res = self.values.get(key);
        if let Some(ParamValue::Str(str_val)) = res {
            return str_val.to_string();
        }
        
        default.to_string()
    }

    pub fn get_value_as_bool(&self, key: &str) -> Option<bool> {
        let res = self.values.get(key);
        if let Some(ParamValue::Bool(val)) = res {
            return Some(*val);
        }
        
        None
    }

    pub fn get_value_as_int(&self, key: &str) -> Option<i32> {
        let res = self.values.get(key);
        if let Some(ParamValue::Int(val)) = res {
            return Some(*val);
        }
        
        None
    }

    pub fn get_values_as_vec_of_strings(&self, key: &str) -> Vec<String> {
        let mut values = Vec::new();
        let res = self.values.get(key);
        if let Some(ParamValue::Array(vec)) = res {
            for it in vec {
                // only strings for the moment...
                if let ParamValue::Str(str) = it {
                    values.push(str.clone());
                }
            }
        }

        values
    }

    pub fn get_raw_value(&self, key: &str) -> Option<&ParamValue> {
        let res = self.values.get(key);
        res
    }
}
