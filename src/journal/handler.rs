use crate::journal::model::{EventTypeData, EventTypeId};
use crate::journal::service::JournalService;
use crate::model::{AppError, IdResponse};
use crate::user::model::UserId;
use actix_web::{web, HttpResponse};

pub async fn find_event_type<T: JournalService>(
    user_id: web::ReqData<UserId>,
    type_id: web::Path<EventTypeId>,
    service: web::Data<T>,
) -> Result<HttpResponse, AppError> {
    service
        .find_event_type_by_id(user_id.into_inner(), type_id.into_inner())
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
    service
        .insert_event_type(user_id.into_inner(), event_type.into_inner())
        .await
        .map(|id| HttpResponse::Ok().json(IdResponse { id }))
}

pub async fn update_event_type<T: JournalService>(
    user_id: web::ReqData<UserId>,
    type_id: web::Path<EventTypeId>,
    event_type: web::Json<EventTypeData>,
    service: web::Data<T>,
) -> Result<HttpResponse, AppError> {
    service
        .update_event_type(user_id.into_inner(), type_id.into_inner(), event_type.into_inner())
        .await
        .map(|_| HttpResponse::Ok().finish())
}

pub async fn delete_event_type<T: JournalService>(
    user_id: web::ReqData<UserId>,
    type_id: web::Path<EventTypeId>,
    service: web::Data<T>,
) -> Result<HttpResponse, AppError> {
    service
        .delete_event_type(user_id.into_inner(), type_id.into_inner())
        .await
        .map(|_| HttpResponse::Ok().finish())
}
