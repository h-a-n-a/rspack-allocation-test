use std::{
    alloc::System,
    borrow::Cow,
    collections::{HashMap, HashSet},
    mem,
    path::{Path, PathBuf},
    sync::{Arc, LazyLock},
    thread::sleep,
    time::Duration,
};

use libmimalloc_sys::{mi_collect, mi_stats_print};
use mimalloc::MiMalloc;
use rspack::builder::{Builder, Devtool};
use rspack_core::{
    BoxLoader, Compiler, Context, EntryDescription, Experiments, Mode, ModuleOptions, ModuleRule,
    ModuleRuleEffect, ModuleRuleUse, ModuleRuleUseLoader, NormalModuleFactoryResolveLoader,
    OutputOptions, Plugin, Resolve, Resolver, RuleSetCondition, BUILTIN_LOADER_PREFIX,
};
use rspack_error::Result;
use rspack_hook::plugin;
use rspack_loader_swc::SwcLoader;
use rspack_macros::plugin_hook;
use rspack_regex::RspackRegex;
use serde_json::json;
use stats_alloc::{Region, StatsAlloc, INSTRUMENTED_SYSTEM};
use tokio::{fs, sync::RwLock};

#[global_allocator]
static GLOBAL: StatsAlloc<MiMalloc> = StatsAlloc::new(MiMalloc);

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
            if let Some(loader) = SWC_LOADER_CACHE.read().await.get(&(
                Cow::Borrowed(loader_request),
                loader_options.clone().unwrap().into(),
            )) {
                return Ok(Some(loader.clone()));
            }

            let loader = Arc::new(
                rspack_loader_swc::SwcLoader::new(loader_options.unwrap())
                    .unwrap()
                    .with_identifier(loader_request.to_string().into()),
            );

            SWC_LOADER_CACHE.write().await.insert(
                (
                    Cow::Owned(loader_request.to_owned()),
                    loader_options.clone().unwrap().into(),
                ),
                loader.clone(),
            );
            return Ok(Some(loader));
        }
    }

    return Ok(None);
}

type SwcLoaderCache<'a> = LazyLock<RwLock<HashMap<(Cow<'a, str>, Arc<str>), Arc<SwcLoader>>>>;
static SWC_LOADER_CACHE: SwcLoaderCache = LazyLock::new(|| RwLock::new(HashMap::default()));

fn rspack() {
    let dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("10000");
    // let dir = PathBuf::from("/home/user/projects/rspack-allocation-test").join("10000");
    // let dir = PathBuf::from("/home/user/projects/rspack-allocation-test").join("10000");
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
    // dbg!(&options);
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
        .plugin(Box::new(BuiltinLoaderRspackPlugin::new_inner()))
        .build();

    // dbg!(&compiler.options, &compiler.plugin_driver.plugins);

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        // .thread_keep_alive(Duration::from_millis(0))
        .build()
        .unwrap();

    rt.block_on(async {
        compiler.build().await.unwrap();
    });

    // rt.shutdown_background();

    eprintln!("Errors: ");
    compiler.compilation.get_errors().for_each(|e| {
        eprintln!("{:#?}", e);
    });

    let mut region = Region::new(&GLOBAL);
    let initial = Region::new(&GLOBAL);

    let mut i = 10;

    loop {
        // let rt = tokio::runtime::Builder::new_multi_thread()
        //     .enable_all()
        //     // .thread_keep_alive(Duration::from_millis(0))
        //     .build()
        //     .unwrap();

        rt.block_on(async {
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

            println!("{:#?}", region.change_and_reset());

            // unsafe { mi_collect(true) };

            sleep(Duration::from_secs(10));
        });

        i -= 1;

        if i == 0 {
            break;
        }
        // rt.shutdown_background();
        let metrics = rt.metrics();
        dbg!(
            metrics.num_alive_tasks(),
            metrics.num_workers(),
            metrics.num_blocking_threads(),
            metrics.num_idle_blocking_threads(),
        );
    }

    drop(compiler);
    //
    println!("initial {:#?}", initial.change());

    sleep(Duration::from_secs(10));
    unsafe {
        mi_stats_print(0 as _);
    }
}

fn mimalloc_reproduce() {}

fn main() {
    // {
    //     rspack();
    // }
    //    let a = vec![1; 1024 * 1024 * 1024];

    // drop(a);
    //   mem::forget(a);
}
