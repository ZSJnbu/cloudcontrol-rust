use actix_web::middleware::ErrorHandlerResponse;
use actix_web::{dev, web, HttpResponse, Result};

/// Render the 404 page using Tera templates.
pub fn handle_404<B>(
    res: dev::ServiceResponse<B>,
) -> Result<ErrorHandlerResponse<B>> {
    let request = res.request().clone();
    let tera = request
        .app_data::<web::Data<tera::Tera>>()
        .cloned();

    let body = if let Some(tera) = tera {
        let ctx = tera::Context::new();
        tera.render("404.html", &ctx).unwrap_or_else(|_| {
            "<h1>404 Not Found</h1>".to_string()
        })
    } else {
        "<h1>404 Not Found</h1>".to_string()
    };

    let new_response = HttpResponse::NotFound()
        .content_type("text/html; charset=utf-8")
        .body(body);

    Ok(ErrorHandlerResponse::Response(
        res.into_response(new_response).map_into_right_body(),
    ))
}

/// Render the 500 page using Tera templates.
pub fn handle_500<B>(
    res: dev::ServiceResponse<B>,
) -> Result<ErrorHandlerResponse<B>> {
    let request = res.request().clone();
    let tera = request
        .app_data::<web::Data<tera::Tera>>()
        .cloned();

    let body = if let Some(tera) = tera {
        let ctx = tera::Context::new();
        tera.render("500.html", &ctx).unwrap_or_else(|_| {
            "<h1>500 Internal Server Error</h1>".to_string()
        })
    } else {
        "<h1>500 Internal Server Error</h1>".to_string()
    };

    let new_response = HttpResponse::InternalServerError()
        .content_type("text/html; charset=utf-8")
        .body(body);

    Ok(ErrorHandlerResponse::Response(
        res.into_response(new_response).map_into_right_body(),
    ))
}
