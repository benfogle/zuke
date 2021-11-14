//! Test components

use crate::options::TestOptions;
use gherkin_rust::{Feature, Rule, Scenario, Step};
use std::fmt;
use std::pin::Pin;
use std::ptr;
use std::sync::Arc;
use thiserror::Error;

/// A test component. Refers to a feature, scenario, step, etc. Used to attach meaning to outcomes.
pub struct Component {
    options: Arc<TestOptions>,
    feature: Option<Pin<Arc<Feature>>>,
    rule: *const Rule,
    scenario: *const Scenario,
    step: *const Step,
    excluded: bool,
    included: bool,
}

impl fmt::Debug for Component {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (kind, name) = match self.kind() {
            ComponentKind::Global => ("global", "-"),
            ComponentKind::Feature => ("feature", self.feature().unwrap().name.as_str()),
            ComponentKind::Rule => ("rule", self.rule().unwrap().name.as_str()),
            ComponentKind::Scenario => ("scenario", self.scenario().unwrap().name.as_str()),
            ComponentKind::Step => ("step", self.step().unwrap().value.as_str()),
        };

        let included = if self.included { "i" } else { "" };
        let excluded = if self.excluded { "e" } else { "" };

        write!(f, "<Component {}:{} {}{}>", kind, name, included, excluded)
    }
}

// we don't access pointers directly.
unsafe impl Sync for Component {}
unsafe impl Send for Component {}

/// The type of test component.
#[derive(Debug, Clone, Copy, PartialOrd, Ord, PartialEq, Eq, Hash)]
pub enum ComponentKind {
    /// Component refers to global scope. No features or scenarios are active.
    Global,
    /// Component refers to a feature.
    Feature,
    /// Component refers to a rule
    Rule,
    /// Component refers to a Scenario, or an example in a Scenario Outline
    Scenario,
    /// Component refers to a step in an executing scenario. Step implementations use this type of
    /// component.
    Step,
}

impl fmt::Display for ComponentKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ComponentKind::Global => "test",
            ComponentKind::Feature => "feature",
            ComponentKind::Rule => "rule",
            ComponentKind::Scenario => "scenario",
            ComponentKind::Step => "step",
        };
        f.write_str(s)
    }
}

/// Errors that can occur when creating a new component
#[derive(Error, Debug)]
pub enum NewComponentError {
    /// No feature is active.
    #[error("No feature")]
    NoFeature,
    /// No scenario is active.
    #[error("No scenario")]
    NoScenario,
    /// Expected a feature level component
    #[error("Expected a feature")]
    ExpectedFeature,
}

impl Component {
    /// The global test options
    pub fn options(&self) -> &Arc<TestOptions> {
        &self.options
    }

    /// The active feature, if applicable.
    pub fn feature(&self) -> Option<&Feature> {
        match self.feature.as_ref() {
            None => None,
            Some(f) => Some(&*f),
        }
    }

    /// The active rule, if applicable.
    pub fn rule(&self) -> Option<&Rule> {
        unsafe { self.rule.as_ref() }
    }

    /// The active scenario, if applicable.
    pub fn scenario(&self) -> Option<&Scenario> {
        unsafe { self.scenario.as_ref() }
    }

    /// The active step, if applicable.
    pub fn step(&self) -> Option<&Step> {
        unsafe { self.step.as_ref() }
    }

    /// The type of component this is.
    pub fn kind(&self) -> ComponentKind {
        if self.step().is_some() {
            ComponentKind::Step
        } else if self.scenario().is_some() {
            ComponentKind::Scenario
        } else if self.rule().is_some() {
            ComponentKind::Rule
        } else if self.feature().is_some() {
            ComponentKind::Feature
        } else {
            ComponentKind::Global
        }
    }

    /// The name of this component, which depends on the type.
    pub fn name(&self) -> &str {
        if let Some(s) = self.step() {
            &s.value
        } else if let Some(s) = self.scenario() {
            &s.name
        } else if let Some(r) = self.rule() {
            &r.name
        } else if let Some(f) = self.feature() {
            &f.name
        } else {
            &self.options.title
        }
    }

    /// The tags for the current component, not including tags inherited from the parent.
    pub fn tags_uninherited(&self) -> &[String] {
        if let Some(s) = self.scenario() {
            &s.tags
        } else if let Some(r) = self.rule() {
            &r.tags
        } else if let Some(f) = self.feature() {
            &f.tags
        } else {
            static EMPTY: [String; 0] = [];
            &EMPTY
        }
    }

    /// The tags for the component, including tags inherited from the parent.
    pub fn tags(&self) -> impl Iterator<Item = &String> {
        // todo: implement more efficiently, if needed
        let n = self.scenario().map(|s| s.tags.len()).unwrap_or(0)
            + self.rule().map(|r| r.tags.len()).unwrap_or(0)
            + self.feature().map(|f| f.tags.len()).unwrap_or(0);

        let mut tags = Vec::with_capacity(n);

        if let Some(s) = self.scenario() {
            tags.extend(s.tags.iter());
        }

        if let Some(r) = self.rule() {
            tags.extend(r.tags.iter());
        }

        if let Some(f) = self.feature() {
            tags.extend(f.tags.iter());
        }

        tags.into_iter()
    }

    /// Is this component excluded by name?
    ///
    /// This component is de-selected, along with everything below it
    pub fn is_excluded(&self) -> bool {
        self.excluded
    }

    /// Is this component included by name
    ///
    /// This component is selected, along with everything below it. If not included, then
    /// sub-componenets may still be selected.
    pub fn is_included(&self) -> bool {
        self.included
    }

    /// Create a global-level component
    pub fn global(options: Arc<TestOptions>) -> Arc<Self> {
        Arc::new(Self {
            options,
            feature: None,
            rule: ptr::null(),
            scenario: ptr::null(),
            step: ptr::null(),
            included: false,
            excluded: false,
        })
    }

    /// Create a feature level component from a global component
    pub fn with_feature(&self, feature: Feature) -> Arc<Self> {
        Arc::new(Self {
            options: self.options.clone(),
            included: self.options.includes(&feature.name),
            excluded: self.options.excludes(&feature.name),
            feature: Some(Arc::pin(feature)),
            rule: ptr::null(),
            scenario: ptr::null(),
            step: ptr::null(),
        })
    }

    /// Create rule level components from a feature component
    pub fn with_rules(&self) -> Result<Vec<Arc<Self>>, NewComponentError> {
        let feature = self.feature.as_ref().ok_or(NewComponentError::NoFeature)?;
        Ok(feature
            .rules
            .iter()
            .map(|rule| {
                Arc::new(Self {
                    options: self.options.clone(),
                    included: self.included || self.options.includes(&rule.name),
                    excluded: self.excluded || self.options.excludes(&rule.name),
                    feature: self.feature.clone(),
                    rule,
                    scenario: ptr::null(),
                    step: ptr::null(),
                })
            })
            .collect())
    }

    /// Create a scenario level component from a feature or rule component.
    /// Doesn't include scenarios inside of Rules, at feature level.
    pub fn with_scenarios(&self) -> Result<Vec<Arc<Self>>, NewComponentError> {
        let feature = self.feature.as_ref().ok_or(NewComponentError::NoFeature)?;

        let scenarios = if let Some(rule) = self.rule() {
            rule.scenarios.iter()
        } else {
            feature.scenarios.iter()
        };

        Ok(scenarios
            .map(|s| {
                Arc::new(Self {
                    options: self.options.clone(),
                    included: self.included || self.options.includes(&s.name),
                    excluded: self.excluded || self.options.excludes(&s.name),
                    feature: self.feature.clone(),
                    rule: self.rule,
                    scenario: s,
                    step: ptr::null(),
                })
            })
            .collect())
    }

    /// Create step level components from a scenario component
    pub fn with_background(&self) -> Result<Vec<Arc<Self>>, NewComponentError> {
        let feature = self.feature().ok_or(NewComponentError::NoFeature)?;
        let mut steps = vec![];

        if let Some(bg) = feature.background.as_ref() {
            steps.extend(bg.steps.iter().map(|s| {
                Arc::new(Self {
                    options: self.options.clone(),
                    included: self.included,
                    excluded: self.excluded,
                    feature: self.feature.clone(),
                    rule: self.rule,
                    scenario: self.scenario,
                    step: s,
                })
            }));
        }

        if let Some(bg) = self.rule().and_then(|r| r.background.as_ref()) {
            steps.extend(bg.steps.iter().map(|s| {
                Arc::new(Self {
                    options: self.options.clone(),
                    included: self.included,
                    excluded: self.excluded,
                    feature: self.feature.clone(),
                    rule: self.rule,
                    scenario: self.scenario,
                    step: s,
                })
            }));
        }

        Ok(steps)
    }

    /// Create step level components from a scenario component
    pub fn with_steps(&self) -> Result<Vec<Arc<Self>>, NewComponentError> {
        self.feature().ok_or(NewComponentError::NoFeature)?;
        let scenario = self.scenario().ok_or(NewComponentError::NoScenario)?;

        Ok(scenario
            .steps
            .iter()
            .map(|s| {
                Arc::new(Self {
                    options: self.options.clone(),
                    included: self.included,
                    excluded: self.excluded,
                    feature: self.feature.clone(),
                    rule: self.rule,
                    scenario: self.scenario,
                    step: s,
                })
            })
            .collect())
    }
}
