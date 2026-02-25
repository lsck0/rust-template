use std::fmt;

use pyroscope::{
    PyroscopeAgent,
    backend::{BackendConfig, PprofConfig, pprof_backend},
    pyroscope::{PyroscopeAgentBuilder, PyroscopeAgentRunning},
};

use crate::{Error, SERVICE_VERSION_ATTRIBUTE, Service, Settings};

const SAMPLE_RATE_HZ: u32 = 100;
const SPY_NAME: &str = "pyroscope-rs";

pub struct Profiler {
    agent: Option<PyroscopeAgent<PyroscopeAgentRunning>>,
}

impl fmt::Debug for Profiler {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        return formatter.debug_struct("Profiler").finish_non_exhaustive();
    }
}

impl Profiler {
    pub fn start(service: Service, address: &str, settings: &Settings) -> Result<Self, Error> {
        let mut builder = PyroscopeAgentBuilder::new(
            address,
            service.name,
            SAMPLE_RATE_HZ,
            SPY_NAME,
            service.version,
            pprof_backend(
                PprofConfig {
                    sample_rate: SAMPLE_RATE_HZ,
                },
                BackendConfig::default(),
            ),
        )
        .tags(vec![(SERVICE_VERSION_ATTRIBUTE, service.version)]);
        if let (Some(user), Some(password)) = (
            &settings.pyroscope_basic_auth_user,
            &settings.pyroscope_basic_auth_password,
        ) {
            builder = builder.basic_auth(user, password);
        }
        if let Some(tenant_id) = &settings.pyroscope_tenant_id {
            builder = builder.tenant_id(tenant_id.clone());
        }
        let agent = builder
            .build()
            .and_then(PyroscopeAgent::start)
            .map_err(Error::Profiler)?;
        return Ok(Self { agent: Some(agent) });
    }
}

impl Drop for Profiler {
    fn drop(&mut self) {
        let Some(agent) = self.agent.take() else {
            return;
        };
        match agent.stop() {
            Ok(agent) => agent.shutdown(),
            Err(error) => eprintln!("telemetry: stopping pyroscope: {error}"),
        }
    }
}
