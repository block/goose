use super::*;
use crate::session::{
    SessionActivity, SessionActivityDay, SessionActivityModel, SessionActivitySession,
};

impl GooseAcpAgent {
    pub(super) async fn on_session_activity(
        &self,
        req: SessionActivityRequest,
    ) -> Result<SessionActivityResponse, agent_client_protocol::Error> {
        if let Some(year) = req.year {
            if !(1970..=2100).contains(&year) {
                return Err(agent_client_protocol::Error::invalid_params()
                    .data("year must be between 1970 and 2100"));
            }
        }
        let activity = self
            .session_manager
            .get_activity(req.year)
            .await
            .internal_err()?;
        Ok(activity_to_response(activity))
    }
}

fn activity_to_response(activity: SessionActivity) -> SessionActivityResponse {
    SessionActivityResponse {
        year: activity.year,
        total_tokens: activity.total_tokens,
        total_sessions: activity.total_sessions as u64,
        days: activity
            .days
            .into_iter()
            .map(activity_day_to_wire)
            .collect(),
        models: activity
            .models
            .into_iter()
            .map(activity_model_to_wire)
            .collect(),
    }
}

fn activity_day_to_wire(
    day: SessionActivityDay,
) -> crate::acp::custom_requests::SessionActivityDay {
    crate::acp::custom_requests::SessionActivityDay {
        date: day.date,
        session_count: day.session_count,
        total_tokens: day.total_tokens,
        sessions: day
            .sessions
            .into_iter()
            .map(activity_session_to_wire)
            .collect(),
    }
}

fn activity_session_to_wire(
    session: SessionActivitySession,
) -> crate::acp::custom_requests::SessionActivitySession {
    crate::acp::custom_requests::SessionActivitySession {
        id: session.id,
        name: session.name,
        total_tokens: session.total_tokens,
        provider_id: session.provider_id,
        model_id: session.model_id,
    }
}

fn activity_model_to_wire(
    model: SessionActivityModel,
) -> crate::acp::custom_requests::SessionActivityModel {
    crate::acp::custom_requests::SessionActivityModel {
        provider_id: model.provider_id,
        model_id: model.model_id,
        total_tokens: model.total_tokens,
        session_count: model.session_count,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::SessionActivitySession as CoreSession;

    #[test]
    fn maps_activity_totals_and_nested_rows() {
        let response = activity_to_response(SessionActivity {
            year: 2024,
            total_tokens: 40,
            total_sessions: 1,
            days: vec![SessionActivityDay {
                date: "2024-01-02".into(),
                session_count: 1,
                total_tokens: 40,
                sessions: vec![CoreSession {
                    id: "s1".into(),
                    name: "Chat".into(),
                    total_tokens: 40,
                    provider_id: Some("openai".into()),
                    model_id: Some("gpt-4o".into()),
                }],
            }],
            models: vec![SessionActivityModel {
                provider_id: Some("openai".into()),
                model_id: Some("gpt-4o".into()),
                total_tokens: 40,
                session_count: 1,
            }],
        });

        assert_eq!(response.year, 2024);
        assert_eq!(response.total_sessions, 1);
        assert_eq!(response.days[0].sessions[0].id, "s1");
        assert_eq!(response.models[0].total_tokens, 40);
    }
}
