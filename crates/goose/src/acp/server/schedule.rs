use goose_sdk_types::custom_requests::{
    CreateScheduleRequest, CreateScheduleResponse, DeleteScheduleRequest, EmptyResponse,
    InspectRunningJobRequest, InspectRunningJobResponse, KillRunningJobRequest,
    KillRunningJobResponse, ListScheduleSessionsRequest, ListScheduleSessionsResponse,
    ListSchedulesRequest, ListSchedulesResponse, PauseScheduleRequest, RunScheduleNowRequest,
    RunScheduleNowResponse, RunScheduleNowStatus, ScheduledJobDto, UnpauseScheduleRequest,
    UpdateScheduleRequest, UpdateScheduleResponse,
};
use tokio::fs;

use super::{build_session_info, GooseAcpAgent, ResultExt};
use crate::recipe::validate_recipe::validate_recipe_template_from_content;
use crate::recipe::Recipe;
use crate::scheduler::{get_default_scheduled_recipes_dir, ScheduledJob, SchedulerError};

fn validate_schedule_id(id: &str) -> Result<(), agent_client_protocol::Error> {
    let is_valid = !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == ' ');

    if !is_valid {
        return Err(agent_client_protocol::Error::invalid_params().data(
            "Schedule name must use only alphanumeric characters, hyphens, underscores, or spaces",
        ));
    }

    Ok(())
}

fn validate_schedule_recipe(recipe: &Recipe) -> Result<(), agent_client_protocol::Error> {
    let recipe_yaml = recipe
        .to_yaml()
        .map_err(|e| agent_client_protocol::Error::invalid_params().data(e.to_string()))?;

    validate_recipe_template_from_content(&recipe_yaml, None)
        .map_err(|e| agent_client_protocol::Error::invalid_params().data(e.to_string()))?;

    Ok(())
}

fn schedule_not_found_or_internal(error: SchedulerError) -> agent_client_protocol::Error {
    match error {
        SchedulerError::JobNotFound(id) => {
            agent_client_protocol::Error::resource_not_found(Some(id))
        }
        error => agent_client_protocol::Error::internal_error().data(error.to_string()),
    }
}

fn create_schedule_error(error: SchedulerError) -> agent_client_protocol::Error {
    match error {
        SchedulerError::CronParseError(message) => agent_client_protocol::Error::invalid_params()
            .data(format!("Invalid cron expression: {message}")),
        SchedulerError::RecipeLoadError(message) => agent_client_protocol::Error::invalid_params()
            .data(format!("Recipe load error: {message}")),
        SchedulerError::JobIdExists(id) => agent_client_protocol::Error::invalid_params()
            .data(format!("Job ID already exists: {id}")),
        error => agent_client_protocol::Error::internal_error()
            .data(format!("Error creating schedule: {error}")),
    }
}

fn schedule_state_error(error: SchedulerError) -> agent_client_protocol::Error {
    match error {
        SchedulerError::JobNotFound(id) => {
            agent_client_protocol::Error::resource_not_found(Some(id))
        }
        SchedulerError::AnyhowError(error) => {
            agent_client_protocol::Error::invalid_params().data(error.to_string())
        }
        error => agent_client_protocol::Error::internal_error().data(error.to_string()),
    }
}

fn update_schedule_error(error: SchedulerError) -> agent_client_protocol::Error {
    match error {
        SchedulerError::JobNotFound(id) => {
            agent_client_protocol::Error::resource_not_found(Some(id))
        }
        SchedulerError::AnyhowError(error) => {
            agent_client_protocol::Error::invalid_params().data(error.to_string())
        }
        SchedulerError::CronParseError(message) => agent_client_protocol::Error::invalid_params()
            .data(format!("Invalid cron expression: {message}")),
        error => agent_client_protocol::Error::internal_error().data(error.to_string()),
    }
}

fn run_schedule_now_error(
    error: SchedulerError,
) -> Result<RunScheduleNowResponse, agent_client_protocol::Error> {
    match error {
        SchedulerError::JobNotFound(id) => {
            Err(agent_client_protocol::Error::resource_not_found(Some(id)))
        }
        SchedulerError::AnyhowError(error)
            if error.to_string().contains("was successfully cancelled") =>
        {
            Ok(RunScheduleNowResponse {
                status: RunScheduleNowStatus::Cancelled,
                session_id: None,
            })
        }
        error => Err(agent_client_protocol::Error::internal_error()
            .data(format!("Error running schedule: {error}"))),
    }
}

fn scheduled_job_to_dto(job: ScheduledJob) -> ScheduledJobDto {
    ScheduledJobDto {
        id: job.id,
        source: job.source,
        cron: job.cron,
        last_run: job.last_run.map(|value| value.to_rfc3339()),
        currently_running: job.currently_running,
        paused: job.paused,
        current_session_id: job.current_session_id,
        job_start_time: job.process_start_time.map(|value| value.to_rfc3339()),
    }
}

impl GooseAcpAgent {
    pub(super) async fn on_list_schedules(
        &self,
        _req: ListSchedulesRequest,
    ) -> Result<ListSchedulesResponse, agent_client_protocol::Error> {
        let jobs = self
            .agent_manager
            .scheduler()
            .list_scheduled_jobs()
            .await
            .into_iter()
            .map(scheduled_job_to_dto)
            .collect();

        Ok(ListSchedulesResponse { jobs })
    }

    pub(super) async fn on_list_schedule_sessions(
        &self,
        req: ListScheduleSessionsRequest,
    ) -> Result<ListScheduleSessionsResponse, agent_client_protocol::Error> {
        let sessions = self
            .agent_manager
            .scheduler()
            .sessions(&req.schedule_id, req.limit)
            .await
            .internal_err_ctx("Failed to fetch schedule sessions")?
            .into_iter()
            .map(|(_, session)| build_session_info(session))
            .collect();

        Ok(ListScheduleSessionsResponse { sessions })
    }

    pub(super) async fn on_create_schedule(
        &self,
        req: CreateScheduleRequest,
    ) -> Result<CreateScheduleResponse, agent_client_protocol::Error> {
        let id = req.id.trim().to_string();
        validate_schedule_id(&id)?;

        let recipe = Recipe::try_from(req.recipe).map_err(|e| {
            agent_client_protocol::Error::invalid_params().data(format!("recipe: {e}"))
        })?;

        if recipe.check_for_security_warnings() {
            return Err(agent_client_protocol::Error::invalid_params().data(
                "This recipe contains hidden characters that could be malicious. Please remove them before trying to save.",
            ));
        }
        validate_schedule_recipe(&recipe)?;

        let scheduled_recipes_dir = get_default_scheduled_recipes_dir().map_err(|e| {
            agent_client_protocol::Error::internal_error()
                .data(format!("Failed to get scheduled recipes directory: {e}"))
        })?;

        let recipe_path = scheduled_recipes_dir.join(format!("{id}.yaml"));
        let yaml_content = recipe.to_yaml().map_err(|e| {
            agent_client_protocol::Error::internal_error()
                .data(format!("Failed to convert recipe to YAML: {e}"))
        })?;
        fs::write(&recipe_path, yaml_content).await.map_err(|e| {
            agent_client_protocol::Error::internal_error()
                .data(format!("Failed to save recipe file: {e}"))
        })?;

        let job = ScheduledJob {
            id,
            source: recipe_path.to_string_lossy().into_owned(),
            cron: req.cron,
            last_run: None,
            currently_running: false,
            paused: false,
            current_session_id: None,
            process_start_time: None,
            parameters: vec![],
            recipe_base_dir: None,
        };

        self.agent_manager
            .scheduler()
            .add_scheduled_job(job.clone(), false)
            .await
            .map_err(create_schedule_error)?;

        Ok(CreateScheduleResponse {
            job: scheduled_job_to_dto(job),
        })
    }

    pub(super) async fn on_delete_schedule(
        &self,
        req: DeleteScheduleRequest,
    ) -> Result<EmptyResponse, agent_client_protocol::Error> {
        self.agent_manager
            .scheduler()
            .remove_scheduled_job(&req.schedule_id, false)
            .await
            .map_err(schedule_not_found_or_internal)?;

        Ok(EmptyResponse {})
    }

    pub(super) async fn on_pause_schedule(
        &self,
        req: PauseScheduleRequest,
    ) -> Result<EmptyResponse, agent_client_protocol::Error> {
        self.agent_manager
            .scheduler()
            .pause_schedule(&req.schedule_id)
            .await
            .map_err(schedule_state_error)?;

        Ok(EmptyResponse {})
    }

    pub(super) async fn on_unpause_schedule(
        &self,
        req: UnpauseScheduleRequest,
    ) -> Result<EmptyResponse, agent_client_protocol::Error> {
        self.agent_manager
            .scheduler()
            .unpause_schedule(&req.schedule_id)
            .await
            .map_err(schedule_not_found_or_internal)?;

        Ok(EmptyResponse {})
    }

    pub(super) async fn on_update_schedule(
        &self,
        req: UpdateScheduleRequest,
    ) -> Result<UpdateScheduleResponse, agent_client_protocol::Error> {
        let schedule_id = req.schedule_id;
        let cron = req.cron;
        let scheduler = self.agent_manager.scheduler();
        scheduler
            .update_schedule(&schedule_id, cron)
            .await
            .map_err(update_schedule_error)?;

        let job = scheduler
            .list_scheduled_jobs()
            .await
            .into_iter()
            .find(|job| job.id == schedule_id)
            .ok_or_else(|| {
                agent_client_protocol::Error::internal_error()
                    .data("Schedule not found after update")
            })?;

        Ok(UpdateScheduleResponse {
            job: scheduled_job_to_dto(job),
        })
    }

    pub(super) async fn on_run_schedule_now(
        &self,
        req: RunScheduleNowRequest,
    ) -> Result<RunScheduleNowResponse, agent_client_protocol::Error> {
        match self
            .agent_manager
            .scheduler()
            .run_now(&req.schedule_id)
            .await
        {
            Ok(session_id) => Ok(RunScheduleNowResponse {
                status: RunScheduleNowStatus::Completed,
                session_id: Some(session_id),
            }),
            Err(error) => run_schedule_now_error(error),
        }
    }

    pub(super) async fn on_kill_running_job(
        &self,
        req: KillRunningJobRequest,
    ) -> Result<KillRunningJobResponse, agent_client_protocol::Error> {
        self.agent_manager
            .scheduler()
            .kill_running_job(&req.job_id)
            .await
            .map_err(schedule_state_error)?;

        Ok(KillRunningJobResponse {
            message: format!("Successfully killed running job '{}'", req.job_id),
        })
    }

    pub(super) async fn on_inspect_running_job(
        &self,
        req: InspectRunningJobRequest,
    ) -> Result<InspectRunningJobResponse, agent_client_protocol::Error> {
        let job = self
            .agent_manager
            .scheduler()
            .list_scheduled_jobs()
            .await
            .into_iter()
            .find(|job| job.id == req.job_id)
            .ok_or_else(|| agent_client_protocol::Error::resource_not_found(Some(req.job_id)))?;

        if !job.currently_running {
            return Ok(InspectRunningJobResponse::default());
        }

        let running_duration_seconds = job.process_start_time.map(|start_time| {
            chrono::Utc::now()
                .signed_duration_since(start_time)
                .num_seconds()
        });

        Ok(InspectRunningJobResponse {
            running: true,
            session_id: job.current_session_id,
            job_start_time: job.process_start_time.map(|value| value.to_rfc3339()),
            running_duration_seconds,
        })
    }
}
