pub mod doctor {
    use crate::actor::BaseActor;

    pub fn pick_to_protect() -> &'static str {
        "Doctor, it's now your turn to pick a player to protect."
    }

    pub fn you_chose_to_protect(target: &BaseActor) -> String {
        format!("You chose to protect {}.", target.name)
    }

    pub fn target_protected(target: &BaseActor) -> String {
        format!(
            "An attempt was made on {}'s life... but they were protected by a doctor!",
            target.name
        )
    }
}

pub mod sheriff {
    use crate::actor::BaseActor;
    use crate::data::roles::RoleAlignment;

    pub fn pick_to_investigate() -> &'static str {
        "Sheriff, it's now your turn to pick a player to investigate."
    }

    pub fn investigate_result(actor: &BaseActor) -> String {
        match actor.role.alignment() {
            RoleAlignment::Mafia => {
                format!("You chose to investigate {}, they are mafia!", actor.name)
            }
            _ => format!(
                "You chose to investigate {}, they are NOT mafia!",
                actor.name
            ),
        }
    }
}

pub mod mafia {
    use crate::actor::BaseActor;

    pub fn build_mafia_list(mafias: &[&BaseActor]) -> String {
        let mut builder = String::from("As a mafia, you know your fellow mafia:");
        for mafia in mafias {
            builder.push_str(&format!("\n{} (ID {})", mafia.name, mafia.id));
        }
        builder
    }

    pub fn mafia_discussion_begin() -> &'static str {
        "Mafia, it's now your turn to discuss. You cannot vote during the discussion--please wait until the discussion ends and the system prompts you to vote."
    }
}
