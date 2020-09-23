// Make sure this is up to date with support article
static TRANSITIONS: &'static [Transition] = &[
    Transition::new("addw2", "PBx0Dx2", false),
    Transition::new("darp6", "N150CU", false), // TODO: set liberate to true
    // 17-inch 1660Ti
    Transition::new_variant("gaze15", 0, "NH5xDC", false),
    // 15-inch 1660Ti
    Transition::new_variant("gaze15", 1, "NH5xDC", false),
    // 17-inch 1650/1650Ti
    Transition::new_variant("gaze15", 2, "NH50DB", false),
    // 15-inch 1650/1650Ti
    Transition::new_variant("gaze15", 3, "NH50DB", false),
];

struct Transition {
    /// Model name
    model: &'static str,
    /// Board variant
    variant: u8,
    /// Open EC project, always "76ec"
    open: &'static str,
    /// Proprietary EC project
    proprietary: &'static str,
    /// If true, TransitionKind::Automatic will switch to open firmware
    liberate: bool,
}

impl Transition {
    const fn new(model: &'static str, proprietary: &'static str, liberate: bool) -> Self {
        Self::new_variant(model, 0, proprietary, liberate)
    }

    const fn new_variant(model: &'static str, variant: u8, proprietary: &'static str, liberate: bool) -> Self {
        Self {
            model,
            variant,
            open: "76ec",
            proprietary,
            liberate
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum TransitionKind {
    /// Whatever the default is
    Automatic,
    /// Open firmware, if available
    Open,
    /// Proprietary firmware, if available
    Proprietary,
}

impl TransitionKind {
    pub fn transition(self, model: &str, variant: u8, project: &str) -> Result<(String, String), String> {
        for transition in TRANSITIONS.iter() {
            if model == transition.model && variant == transition.variant {
                let new_project = if project == transition.open {
                    match self {
                        TransitionKind::Automatic => transition.open,
                        TransitionKind::Open => transition.open,
                        TransitionKind::Proprietary => transition.proprietary,
                    }
                } else if project == transition.proprietary {
                    match self {
                        TransitionKind::Automatic => if transition.liberate {
                            transition.open
                        } else {
                            transition.proprietary
                        },
                        TransitionKind::Open => transition.open,
                        TransitionKind::Proprietary => transition.proprietary,
                    }
                } else {
                    project
                };

                eprintln!("{:?} transition: {} -> {}", self, project, new_project);

                return Ok((model.to_string(), new_project.to_string()));
            }
        }

        eprintln!("{:?} transition: {} -> {}", self, project, project);

        // Fallback in case transition is not defined
        match self {
            TransitionKind::Open if project != "76ec" => {
                Err(format!("Model '{}' is not supported by open firmware and EC at this time", model))
            },
            TransitionKind::Proprietary if project == "76ec" => {
                Err(format!("Model '{}' is not supported by proprietary firmware and EC at this time", model))
            },
            _ => Ok((model.to_string(), project.to_string())),
        }

    }
}
