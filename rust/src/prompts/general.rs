use crate::actor::BaseActor;
use crate::data::roles::GameRole;
use crate::game::EndResult;

pub fn utter_beginning(actor_count: u8, extra_messages: u8) -> String {
    format!(
        include_str!("utter_beginning.txt"),
        actor_count, extra_messages
    )
}

pub fn introduce_you(actor: &BaseActor) -> String {
    format!(
        "You are {} (ID {}) with the role of {}: {}",
        actor.name,
        actor.id,
        actor.role.name(),
        actor.role.description()
    )
}

pub fn build_role_list(roles: &[&GameRole]) -> String {
    let mut builder = String::from("The roles in the game are:");
    for role in roles {
        builder.push_str(&format!("\n{}: {}", role.name(), role.description()));
    }
    builder
}

pub fn build_actor_list(actors: &[BaseActor]) -> String {
    let mut builder = String::from("The players in the game are:");
    for actor in actors {
        builder.push_str(&format!("\n{} (ID {})", actor.name, actor.id));
    }
    builder
}

pub fn actor_was_killed(actor: &BaseActor) -> String {
    format!(
        "{} was killed! They were a {}.",
        actor.name,
        actor.role.name()
    )
}

pub fn day_time(day_count: u8) -> String {
    if day_count == 0 {
        format!(
            "It is now day {}--the first day. After discussion ends, voting will begin--remember that you cannot vote during the discussion.",
            day_count
        )
    } else {
        format!(
            "It is now day {}. After discussion ends, voting will begin--remember that you cannot vote during the discussion.",
            day_count
        )
    }
}

pub fn night_time(night_count: u8) -> String {
    if night_count == 0 {
        format!(
            "It is now night {}--the first night. Night acting groups will make their move now.",
            night_count
        )
    } else {
        format!(
            "It is now night {}. Night acting groups will make their move now.",
            night_count
        )
    }
}

pub fn your_turn_to_talk(actor: &BaseActor, core_messages: u8, extra_messages: u8) -> String {
    format!(
        "It's now your turn to talk. There are currently {} core messages and {} extra messages. Remember: you are {}, a {}.",
        core_messages,
        extra_messages,
        actor.name,
        actor.role.name()
    )
}

pub fn tagged_for_comment(tagger: &BaseActor, tagged: &BaseActor) -> String {
    format!("{} tagged {} for comment.", tagger.name, tagged.name)
}

pub fn voting_begins() -> &'static str {
    "Voting has begun."
}

pub fn time_to_vote() -> &'static str {
    "It's now your turn to vote. You can abstain or use the Talk tool to leave a comment or explanation if you wish."
}

pub fn actor_voted(
    voter: &BaseActor,
    voted: Option<&BaseActor>,
    comment: Option<String>,
) -> String {
    match (voted, comment) {
        (Some(v), Some(c)) => format!("{} voted for {} with a comment: {}", voter.name, v.name, c),
        (None, Some(c)) => format!("{} abstained with a comment: {}", voter.name, c),
        (Some(v), None) => format!("{} voted for {} with no comment.", voter.name, v.name),
        (None, None) => format!("{} abstained with no comment.", voter.name),
    }
}

pub fn voting_ends(actor: Option<&BaseActor>, reveal_role: bool) -> String {
    if let Some(actor) = actor {
        if reveal_role {
            format!(
                "Voting has ended. {} received the most votes. They were a {}.",
                actor.name,
                actor.role.name()
            )
        } else {
            format!("Voting has ended. {} received the most votes.", actor.name)
        }
    } else {
        "Voting has ended. Nobody in particular received the most votes.".to_string()
    }
}

pub fn abstained_in_discussion(actor: &BaseActor) -> String {
    format!("{} abstained", actor.name)
}

pub fn public_whisper_notice(from: &BaseActor, to: &BaseActor) -> String {
    format!("{} whispered to {}", from.name, to.name)
}

pub fn whisperer(to: &BaseActor, message: &str) -> String {
    format!("You whispered to {}: {}", to.name, message)
}

pub fn whispered(from: &BaseActor, message: &str) -> String {
    format!(
        "{} (ID {}) whispered to you: {}",
        from.name, from.id, message
    )
}

pub fn game_end(end_result: &EndResult) -> String {
    let mut builder = String::from("The game has ended.\n");
    match end_result {
        EndResult::Mafia => {
            builder.push_str("The mafia won--they reached an equal amount of players with town")
        }
        EndResult::Town => builder.push_str("The town won--all mafia were eliminated"),
    }
    builder
}
