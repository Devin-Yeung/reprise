use std::cell::RefCell;
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::PathBuf;

use super::{Environment, IntegrationPlanRequest, OutputResolver, RUNSC_ENV, RUNTIME_ENV};

#[test]
fn builder_prefers_explicit_environment_without_resolving_outputs() {
    let environment = TestEnvironment::from([
        (RUNSC_ENV, "/custom/runsc"),
        (RUNTIME_ENV, "/custom/runtime"),
    ]);
    let outputs = TestOutputs::default();

    let plan = IntegrationPlanRequest::builder()
        .executable(PathBuf::from("/test/integration"))
        .extra_args(vec!["--nocapture".to_owned()])
        .environment(&environment)
        .outputs(&outputs)
        .build()
        .resolve()
        .expect("explicit environment should build a plan");

    assert_eq!(
        plan.env,
        vec![
            (RUNSC_ENV.to_owned(), OsString::from("/custom/runsc")),
            (RUNTIME_ENV.to_owned(), OsString::from("/custom/runtime")),
        ]
    );
    assert_eq!(plan.extra_args, ["--nocapture"]);
    assert!(outputs.resolved.borrow().is_empty());
}

#[test]
fn builder_resolves_missing_environment_from_nix_outputs() {
    let environment = TestEnvironment::default();
    let outputs = TestOutputs::from([
        (".#runsc", "/nix/store/runsc"),
        (".#reprise-test-runtime", "/nix/store/runtime"),
    ]);

    let plan = IntegrationPlanRequest::builder()
        .executable(PathBuf::from("/test/integration"))
        .environment(&environment)
        .outputs(&outputs)
        .build()
        .resolve()
        .expect("nix outputs should build a plan");

    assert_eq!(
        plan.env,
        vec![
            (
                RUNSC_ENV.to_owned(),
                OsString::from("/nix/store/runsc/bin/runsc"),
            ),
            (RUNTIME_ENV.to_owned(), OsString::from("/nix/store/runtime")),
        ]
    );
    assert_eq!(
        outputs.resolved.into_inner(),
        [".#runsc", ".#reprise-test-runtime"]
    );
}

#[derive(Default)]
struct TestEnvironment(BTreeMap<&'static str, OsString>);

impl<const N: usize> From<[(&'static str, &'static str); N]> for TestEnvironment {
    fn from(values: [(&'static str, &'static str); N]) -> Self {
        Self(
            values
                .into_iter()
                .map(|(name, value)| (name, OsString::from(value)))
                .collect(),
        )
    }
}

impl Environment for TestEnvironment {
    fn var(&self, name: &str) -> Option<OsString> {
        self.0.get(name).cloned()
    }
}

#[derive(Default)]
struct TestOutputs {
    paths: BTreeMap<&'static str, PathBuf>,
    resolved: RefCell<Vec<&'static str>>,
}

impl<const N: usize> From<[(&'static str, &'static str); N]> for TestOutputs {
    fn from(values: [(&'static str, &'static str); N]) -> Self {
        Self {
            paths: values
                .into_iter()
                .map(|(reference, path)| (reference, PathBuf::from(path)))
                .collect(),
            resolved: RefCell::default(),
        }
    }
}

impl OutputResolver for TestOutputs {
    fn resolve(&self, flake_ref: &str) -> std::io::Result<PathBuf> {
        let path = self
            .paths
            .get(flake_ref)
            .expect("test must define every resolved output")
            .clone();
        self.resolved.borrow_mut().push(match flake_ref {
            ".#runsc" => ".#runsc",
            ".#reprise-test-runtime" => ".#reprise-test-runtime",
            _ => unreachable!("test only resolves known outputs"),
        });
        Ok(path)
    }
}
