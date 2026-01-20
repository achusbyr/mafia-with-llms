#[derive(Debug, Clone)]
pub enum GameRole {
    Villager,
    Mafioso,
    Doctor,
    Sheriff,
}

pub enum RoleAlignment {
    Town,
    Mafia,
}

impl GameRole {
    pub fn name(&self) -> String {
        format!("{:?}", self)
    }

    pub fn alignment(&self) -> RoleAlignment {
        match self {
            GameRole::Mafioso => RoleAlignment::Mafia,
            _ => RoleAlignment::Town,
        }
    }

    pub fn description(&self) -> &str {
        match self {
            GameRole::Villager => "Discuss with others during the day and figure out the mafia.",
            GameRole::Mafioso => {
                "Discuss with fellow mafia at night to plan and vote a player to kill."
            }
            GameRole::Doctor => "At night, pick a person to protect from being killed.",
            GameRole::Sheriff => "At night, investigate a player and see if they are mafia.",
        }
    }
}
