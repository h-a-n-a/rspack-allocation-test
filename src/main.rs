use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::Arc,
    thread::sleep,
    time::Duration,
};

use libmimalloc_sys::mi_stats_print;
use mimalloc::MiMalloc;
use rspack::builder::{Builder, Devtool};
use rspack_core::{
    BUILTIN_LOADER_PREFIX, BoxLoader, Compiler, Context, EntryDescription, Experiments, Mode,
    ModuleOptions, ModuleRule, ModuleRuleEffect, ModuleRuleUse, ModuleRuleUseLoader,
    NormalModuleFactoryResolveLoader, OutputOptions, Plugin, Resolve, Resolver, RuleSetCondition,
};
use rspack_error::Result;
use rspack_hook::plugin;
use rspack_macros::plugin_hook;
use rspack_regex::RspackRegex;
use serde_json::json;
use tokio::fs;

// #[global_allocator]
// static GLOBAL: MiMalloc = MiMalloc;

fn bulk() {
    loop {
        println!("Hello");
        let mut v = vec![1; 1024 * 1024 * 1024];
        unsafe {
            mi_stats_print(0 as _);
        }
        sleep(Duration::from_secs(2));
        std::thread::spawn(move || {
            drop(v);
        })
        .join()
        .unwrap();
        unsafe {
            mi_stats_print(0 as _);
        }
        sleep(Duration::from_secs(2));

        println!("=======================================");
        println!("=======================================");
        println!("=======================================");
        println!("=======================================");
    }
}

#[plugin]
#[derive(Debug)]
pub struct BuiltinLoaderRspackPlugin;

impl Plugin for BuiltinLoaderRspackPlugin {
    fn name(&self) -> &'static str {
        "BuiltinLoaderRspackPlugin"
    }
    fn apply(
        &self,
        ctx: rspack_core::PluginContext<&mut rspack_core::ApplyContext>,
        _options: &rspack_core::CompilerOptions,
    ) -> Result<()> {
        ctx.context
            .normal_module_factory_hooks
            .resolve_loader
            .tap(resolve_loader::new(self));
        Ok(())
    }
}

#[plugin_hook(NormalModuleFactoryResolveLoader for BuiltinLoaderRspackPlugin)]
pub(crate) async fn resolve_loader(
    &self,
    context: &Context,
    resolver: &Resolver,
    l: &ModuleRuleUseLoader,
) -> Result<Option<BoxLoader>> {
    let context = context.as_path();
    let loader_request = &l.loader;
    let loader_options = l.options.as_deref();

    // FIXME: not belong to napi
    if loader_request.starts_with(BUILTIN_LOADER_PREFIX) {
        if loader_request.starts_with("builtin:swc-loader") {
            return Ok(Some(Arc::new(
                rspack_loader_swc::SwcLoader::new(loader_options.unwrap())
                    .unwrap()
                    .with_identifier(loader_request.to_string().into()),
            )));
        }
    }

    return Ok(None);
}

#[tokio::main]
async fn rspack() {
    // let dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("10000");
    // let dir = PathBuf::from("/Users/bytedance/Projects/mimalloc-test").join("10000");
    let dir = PathBuf::from("/home/user/projects/rspack-allocation-test").join("10000");
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
    dbg!(&options);
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
        .resolve(Resolve {
            extensions: Some(vec!["...".to_string(), ".jsx".to_string()]),
            ..Default::default()
        })
        .experiments(Experiments::builder().css(true))
        .plugin(Box::new(BuiltinLoaderRspackPlugin::new_inner()))
        .build();

    dbg!(&compiler.options, &compiler.plugin_driver.plugins);

    compiler.build().await.unwrap();

    eprintln!("Errors: ");
    compiler.compilation.get_errors().for_each(|e| {
        eprintln!("{:#?}", e);
    });

    let mut i = 10;

    loop {
        let mut content = std::fs::read(dir.join("index.jsx")).unwrap();
        content.extend(b"\nconsole.log('Hello, world!');");
        let _ = std::fs::write(dir.join("index.jsx"), content).unwrap();

        compiler
            .rebuild(
                HashSet::from_iter(std::iter::once(
                    dir.join("index.jsx").to_string_lossy().to_string(),
                )),
                HashSet::default(),
            )
            .await
            .unwrap();

        unsafe {
            mi_stats_print(0 as _);
        }

        sleep(Duration::from_secs(2));

        i -= 1;

        if i == 0 {
            break;
        }
    }

    drop(compiler);

    sleep(Duration::from_secs(2));

    unsafe {
        mi_stats_print(0 as _);
    }
}

fn main() {
    rspack();
}
