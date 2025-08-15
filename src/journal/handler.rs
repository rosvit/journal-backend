use crate::journal::model::{
    EventTypeData, EventTypeId, JournalEntryId, JournalEntryUpdate, NewJournalEntry, SearchFilter,
};
use crate::journal::service::JournalService;
use crate::model::{AppError, IdResponse};
use crate::user::model::UserId;
use actix_web::{HttpResponse, web};
use validator::Validate;

pub async fn find_event_type<T: JournalService>(
    user_id: web::ReqData<UserId>,
    id: web::Path<EventTypeId>,
    service: web::Data<T>,
) -> Result<HttpResponse, AppError> {
    service
        .find_event_type_by_id(user_id.into_inner(), id.into_inner())
        .await
        .map(|et| HttpResponse::Ok().json(et))
}

pub async fn find_user_event_types<T: JournalService>(
    user_id: web::ReqData<UserId>,
    service: web::Data<T>,
) -> Result<HttpResponse, AppError> {
    service.find_all_event_types(user_id.into_inner()).await.map(|et| HttpResponse::Ok().json(et))
}

pub async fn insert_event_type<T: JournalService>(
    user_id: web::ReqData<UserId>,
    event_type: web::Json<EventTypeData>,
    service: web::Data<T>,
) -> Result<HttpResponse, AppError> {
    let event_type = event_type.into_inner();
    event_type.validate().map_err(AppError::from)?;
    service
        .insert_event_type(user_id.into_inner(), event_type)
        .await
        .map(|id| HttpResponse::Ok().json(IdResponse { id }))
}

pub async fn update_event_type<T: JournalService>(
    user_id: web::ReqData<UserId>,
    id: web::Path<EventTypeId>,
    event_type: web::Json<EventTypeData>,
    service: web::Data<T>,
) -> Result<HttpResponse, AppError> {
    let event_type = event_type.into_inner();
    event_type.validate().map_err(AppError::from)?;
    service
        .update_event_type(user_id.into_inner(), id.into_inner(), event_type)
        .await
        .map(|_| HttpResponse::Ok().finish())
}

pub async fn delete_event_type<T: JournalService>(
    user_id: web::ReqData<UserId>,
    id: web::Path<EventTypeId>,
    service: web::Data<T>,
) -> Result<HttpResponse, AppError> {
    service
        .delete_event_type(user_id.into_inner(), id.into_inner())
        .await
        .map(|_| HttpResponse::Ok().finish())
}

pub async fn find_journal_entry<T: JournalService>(
    user_id: web::ReqData<UserId>,
    id: web::Path<JournalEntryId>,
    service: web::Data<T>,
) -> Result<HttpResponse, AppError> {
    service
        .find_journal_entry_by_id(user_id.into_inner(), id.into_inner())
        .await
        .map(|et| HttpResponse::Ok().json(et))
}

pub async fn find_journal_entries<T: JournalService>(
    user_id: web::ReqData<UserId>,
    filter: web::Query<SearchFilter>,
    service: web::Data<T>,
) -> Result<HttpResponse, AppError> {
    let filter = filter.into_inner();
    filter.validate().map_err(AppError::from)?;
    service
        .find_journal_entries(user_id.into_inner(), filter)
        .await
        .map(|et| HttpResponse::Ok().json(et))
}

pub async fn insert_journal_entry<T: JournalService>(
    user_id: web::ReqData<UserId>,
    entry: web::Json<NewJournalEntry>,
    service: web::Data<T>,
) -> Result<HttpResponse, AppError> {
    let entry = entry.into_inner();
    entry.validate().map_err(AppError::from)?;
    service
        .insert_journal_entry(user_id.into_inner(), entry)
        .await
        .map(|id| HttpResponse::Ok().json(IdResponse { id }))
}

pub async fn update_journal_entry<T: JournalService>(
    user_id: web::ReqData<UserId>,
    id: web::Path<JournalEntryId>,
    entry: web::Json<JournalEntryUpdate>,
    service: web::Data<T>,
) -> Result<HttpResponse, AppError> {
    let entry = entry.into_inner();
    entry.validate().map_err(AppError::from)?;
    service
        .update_journal_entry(user_id.into_inner(), id.into_inner(), entry)
        .await
        .map(|_| HttpResponse::Ok().finish())
}

pub async fn delete_journal_entry<T: JournalService>(
    user_id: web::ReqData<UserId>,
    id: web::Path<JournalEntryId>,
    service: web::Data<T>,
) -> Result<HttpResponse, AppError> {
    service
        .delete_journal_entry(user_id.into_inner(), id.into_inner())
        .await
        .map(|_| HttpResponse::Ok().finish())
}
