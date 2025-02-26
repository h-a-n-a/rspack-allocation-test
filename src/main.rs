use std::{collections::HashSet, path::PathBuf, thread::sleep, time::Duration};

use libmimalloc_sys::mi_stats_print;
use mimalloc::MiMalloc;
use rspack::builder::{Builder, Devtool};
use rspack_core::{
    Compiler, Experiments, Mode, ModuleOptions, ModuleRule, ModuleRuleEffect, ModuleRuleUse,
    ModuleRuleUseLoader, Resolve, RuleSetCondition,
};
use rspack_regex::RspackRegex;
use serde_json::json;
// use stats_alloc::StatsAlloc;

// #[global_allocator]
// static GLOBAL: StatsAlloc<MiMalloc> = StatsAlloc::new(MiMalloc);

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn rspack() {
    let dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("10000");
    let options = json!({
        "jsc": {
            "parser": {
                "syntax": "typescript",
                "tsx": true,
            },
            "transform": {
                "react": {
                    "runtime": "automatic",
                },
            },
            "externalHelpers": true,
        },
        "env": {
          "targets": "Chrome >= 48"
        }
    })
    .to_string();

    let mut compiler = Compiler::builder()
        .context(dir.to_string_lossy().to_string())
        .mode(Mode::Development)
        .devtool(Devtool::False)
        .entry("main", "./index.jsx")
        .module(ModuleOptions::builder().rule(ModuleRule {
            test: Some(RuleSetCondition::Regexp(
                RspackRegex::new("\\.(j|t)s(x)?$").unwrap(),
            )),
            effect: ModuleRuleEffect {
                r#use: ModuleRuleUse::Array(vec![ModuleRuleUseLoader {
                    loader: "builtin:swc-loader".to_string(),
                    options: Some(options),
                }]),
                ..Default::default()
            },
            ..Default::default()
        }))
        .cache(rspack_core::CacheOptions::Disabled)
        .resolve(Resolve {
            extensions: Some(vec!["...".to_string(), ".jsx".to_string()]),
            ..Default::default()
        })
        .experiments(Experiments::builder().css(true))
        .enable_loader_swc()
        .build();

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    rt.block_on(async {
        compiler.build().await.unwrap();
    });

    eprintln!("Errors: ");
    compiler.compilation.get_errors().for_each(|e| {
        eprintln!("{:#?}", e);
    });

    let mut i = 20;

    loop {
        rt.block_on(async {
            let mut content = std::fs::read(dir.join("index.jsx")).unwrap();
            content.extend(b"\nconsole.log('Hello, world!');");
            std::fs::write(dir.join("index.jsx"), content).unwrap();

            compiler
                .rebuild(
                    HashSet::from_iter(std::iter::once(
                        dir.join("index.jsx").to_string_lossy().to_string(),
                    )),
                    HashSet::default(),
                )
                .await
                .unwrap();

            println!("Rebuild count: {}", i);
            unsafe {
                mi_stats_print(0 as _);
            }

            sleep(Duration::from_secs(5));
        });

        i -= 1;

        if i == 0 {
            break;
        }
        // rt.shutdown_background();
        // let metrics = rt.metrics();
        // dbg!(
        //     metrics.num_alive_tasks(),
        //     metrics.num_workers(),
        //     metrics.num_blocking_threads(),
        //     metrics.num_idle_blocking_threads(),
        // );
    }

    drop(compiler);

    unsafe {
        mi_stats_print(0 as _);
    }
}

fn main() {
    rspack()
}
