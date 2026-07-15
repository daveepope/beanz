#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Leniency {
    Lenient,
    Normal,
    Strict,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WeightProfile {
    pub truncation: f64,
    pub middle: f64,
    pub spec_gap: f64,
    pub cyclomatic: f64,
    pub structural: f64,
    pub bytes: f64,
    pub probe_relief: f64,
    pub suggestion_threshold: f64,
}

impl Leniency {
    pub fn profile(self) -> WeightProfile {
        match self {
            Leniency::Lenient => WeightProfile {
                truncation: 0.75,
                middle: 0.75,
                spec_gap: 0.8,
                cyclomatic: 0.85,
                structural: 0.85,
                bytes: 0.85,
                probe_relief: 1.3,
                suggestion_threshold: 1.25,
            },
            Leniency::Normal => WeightProfile::normal(),
            Leniency::Strict => WeightProfile {
                truncation: 1.25,
                middle: 1.25,
                spec_gap: 1.2,
                cyclomatic: 1.15,
                structural: 1.15,
                bytes: 1.15,
                probe_relief: 0.7,
                suggestion_threshold: 0.8,
            },
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Leniency::Lenient => "lenient",
            Leniency::Normal => "normal",
            Leniency::Strict => "strict",
        }
    }
}

impl WeightProfile {
    pub fn normal() -> Self {
        Self {
            truncation: 1.0,
            middle: 1.0,
            spec_gap: 1.0,
            cyclomatic: 1.0,
            structural: 1.0,
            bytes: 1.0,
            probe_relief: 1.0,
            suggestion_threshold: 1.0,
        }
    }
}

pub fn resolve_leniency(cli_lenient: bool, cli_strict: bool) -> Result<Leniency, String> {
    resolve_leniency_inputs(
        cli_lenient,
        cli_strict,
        env_enabled("BEANZ_LENIENT"),
        env_enabled("BEANZ_STRICT"),
    )
}

pub fn resolve_leniency_inputs(
    cli_lenient: bool,
    cli_strict: bool,
    env_lenient: bool,
    env_strict: bool,
) -> Result<Leniency, String> {
    if cli_lenient && cli_strict {
        return Err("cannot use --lenient and --strict together".to_string());
    }

    if env_lenient && env_strict {
        return Err("BEANZ_LENIENT and BEANZ_STRICT cannot both be set".to_string());
    }

    if cli_lenient {
        if env_strict {
            return Err("BEANZ_STRICT contradicts --lenient".to_string());
        }
        return Ok(Leniency::Lenient);
    }

    if cli_strict {
        if env_lenient {
            return Err("BEANZ_LENIENT contradicts --strict".to_string());
        }
        return Ok(Leniency::Strict);
    }

    if env_lenient {
        return Ok(Leniency::Lenient);
    }

    if env_strict {
        return Ok(Leniency::Strict);
    }

    Ok(Leniency::Normal)
}

fn env_enabled(name: &str) -> bool {
    std::env::var(name)
        .map(|value| {
            let value = value.trim();
            !value.is_empty() && value != "0" && !value.eq_ignore_ascii_case("false")
        })
        .unwrap_or(false)
}
