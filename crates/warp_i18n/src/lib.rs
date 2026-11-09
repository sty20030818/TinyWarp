//! TinyWarp 的内嵌简体中文文案；缺译或目录损坏时保留源码英文。

use std::collections::HashMap;
use std::fmt;
use std::sync::OnceLock;

use serde::Deserialize;
use serde::de::{self, MapAccess, Visitor};

const EMBEDDED_CATALOG: &str = include_str!("../resources/zh-CN.json");
static CATALOG: OnceLock<Catalog> = OnceLock::new();

/// 查询固定简体中文文案；缺译时返回调用点的英文常量。
pub fn tr(key: &str, english: &'static str) -> &'static str {
    CATALOG
        .get_or_init(|| load_catalog(EMBEDDED_CATALOG))
        .lookup(key, english)
}

#[derive(Default)]
struct Catalog(HashMap<String, String>);

impl Catalog {
    fn parse(source: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(source)
    }

    fn lookup<'a>(&'a self, key: &str, english: &'static str) -> &'a str {
        self.0.get(key).map_or(english, String::as_str)
    }
}

fn load_catalog(source: &str) -> Catalog {
    Catalog::parse(source).unwrap_or_default()
}

impl<'de> Deserialize<'de> for Catalog {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct CatalogVisitor;

        impl<'de> Visitor<'de> for CatalogVisitor {
            type Value = Catalog;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("文案键到非空译文的 JSON 对象")
            }

            fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<Self::Value, M::Error> {
                let mut entries = HashMap::new();
                while let Some((key, value)) = map.next_entry::<String, String>()? {
                    if key.trim().is_empty() || value.trim().is_empty() {
                        return Err(de::Error::custom("文案键和译文不能为空"));
                    }
                    if entries.insert(key, value).is_some() {
                        return Err(de::Error::custom("文案键重复"));
                    }
                }
                Ok(Catalog(entries))
            }
        }

        deserializer.deserialize_map(CatalogVisitor)
    }
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
