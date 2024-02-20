/// Refer to this url for more information: https://developer.mozilla.org/en-US/docs/Web/HTTP/Status
#[derive(Debug)]
pub enum Status {
    // Information responses
    Continue = 100,
    SwitchingProtocols = 101,
    Processing = 102,
    EarlyHints = 103,

    // Successful responses
    Ok = 200,
    Created = 201,
    Accepted = 202,
    NonAuthoritativeInformation = 203,
    NoContent = 204,
    ResetContent = 205,
    PartialContent = 206,
    MultiStatus = 207,
    AlreadyReported = 208,
    ImUsed = 226,

    // Redirection Messages
    MultipleChoices = 300,
    MovedPermanently = 301,
    Found = 302,
    SeeOther = 303,
    NotModified = 304,
    /// Depreciated
    UseProxy = 305,
    /// Depreciated
    UnUsed = 306,
    /// No longer used, just reserved
    TemporaryRedirect = 307,
    PermanentRedirect = 308,

    // Client error responses
    BadRequest = 400,
    UnAuthorized = 401,
    /// Experimental. Expect behaviour to change in the future.
    PaymentRequired = 402,
    Forbidden = 403,
    NotFound = 404,
    MethodNotAllowed = 405,
    NotAcceptable = 406,
    ProxyAuthenticationRequired = 407,
    RequestTimeout = 408,
    Conflict = 409,
    Gone = 410,
    LengthRequired = 411,
    PreconditionFailed = 412,
    PayloadTooLarge = 413,
    UriTooLong = 414,
    UnsupportedMediaType = 415,
    RangeNotSatisfiable = 416,
    ExpectationFailed = 417,
    ImaTeaPot = 418,
    MisRedirectRequest = 421,
    UnprocessableContent = 422,
    Locked = 423,
    FailedDependency = 424,
    TooEarly = 425,
    UpgradeRequired = 426,
    PreconditionRequired = 428,
    TooManyRequests = 429,
    RequestHeaderFieldsTooLarge = 431,
    UnavailableForLegalReasons = 451,

    // Server error responses
    InternalServerError = 500,
    NotImplemented = 501,
    BadGateway = 502,
    ServiceUnavailable = 503,
    GatewayTimeout = 504,
    HttpVersionNotSupported = 505,
    VariantAlsoNegotiates = 506,
    InsufficientStorage = 507,
    LoopDetected = 508,
    NotExtended = 510,
    NetworkAuthenticationRequired = 511,
}

pub trait StatusMethods {
    fn status_code(&self) -> Option<usize>;
    fn status_text(status_code: usize) -> Option<String>;
}

impl StatusMethods for Status {
    fn status_code(&self) -> Option<usize> {
        match self {
            // Information responses
            Status::Continue => Some(100),
            Status::SwitchingProtocols => Some(101),
            Status::Processing => Some(102),
            Status::EarlyHints => Some(103),

            // Successful responses
            Status::Ok => Some(200),
            Status::Created => Some(201),
            Status::Accepted => Some(202),
            Status::NonAuthoritativeInformation => Some(203),
            Status::NoContent => Some(204),
            Status::ResetContent => Some(205),
            Status::PartialContent => Some(206),
            Status::MultiStatus => Some(207),
            Status::AlreadyReported => Some(208),
            Status::ImUsed => Some(226),

            // Redirection Messages
            Status::MultipleChoices => Some(300),
            Status::MovedPermanently => Some(301),
            Status::Found => Some(302),
            Status::SeeOther => Some(303),
            Status::NotModified => Some(304),
            Status::UseProxy => Some(305), // Depreciated
            Status::UnUsed => Some(306), // Depreciated
            Status::TemporaryRedirect => Some(307), // No longer used, just reserved
            Status::PermanentRedirect => Some(308),

            // Client error responses
            Status::BadRequest => Some(400),
            Status::UnAuthorized => Some(401), // Experimental. Expect behaviour to change in the future.
            Status::PaymentRequired => Some(402),
            Status::Forbidden => Some(403),
            Status::NotFound => Some(404),
            Status::MethodNotAllowed => Some(405),
            Status::NotAcceptable => Some(406),
            Status::ProxyAuthenticationRequired => Some(407),
            Status::RequestTimeout => Some(408),
            Status::Conflict => Some(409),
            Status::Gone => Some(410),
            Status::LengthRequired => Some(411),
            Status::PreconditionFailed => Some(412),
            Status::PayloadTooLarge => Some(413),
            Status::UriTooLong => Some(414),
            Status::UnsupportedMediaType => Some(415),
            Status::RangeNotSatisfiable => Some(416),
            Status::ExpectationFailed => Some(417),
            Status::ImaTeaPot => Some(418),
            Status::MisRedirectRequest => Some(421),
            Status::UnprocessableContent => Some(422),
            Status::Locked => Some(423),
            Status::FailedDependency => Some(424),
            Status::TooEarly => Some(425), // Experimental. Expect behaviour to change in the future.
            Status::UpgradeRequired => Some(426),
            Status::PreconditionRequired => Some(428),
            Status::TooManyRequests => Some(429),
            Status::RequestHeaderFieldsTooLarge => Some(431),
            Status::UnavailableForLegalReasons => Some(451),

            // Server error responses
            Status::InternalServerError => Some(500),
            Status::NotImplemented => Some(501),
            Status::BadGateway => Some(502),
            Status::ServiceUnavailable => Some(503),
            Status::GatewayTimeout => Some(504),
            Status::HttpVersionNotSupported => Some(505),
            Status::VariantAlsoNegotiates => Some(506),
            Status::InsufficientStorage => Some(507),
            Status::LoopDetected => Some(508),
            Status::NotExtended => Some(510),
            Status::NetworkAuthenticationRequired => Some(511)
        }
    }

    fn status_text(status_code: usize) -> Option<String> {
        return match status_code {
            // Information responses
            100 => Some("Continue".to_string()),
            101 => Some("Switching Protocols".to_string()),
            102 => Some("Processing".to_string()),
            103 => Some("Early Hints".to_string()),

            // Successful responses
            200 => Some("OK".to_string()),
            201 => Some("Created".to_string()),
            202 => Some("Accepted".to_string()),
            203 => Some("Non-Authoritative Information".to_string()),
            204 => Some("No Content".to_string()),
            205 => Some("Reset Content".to_string()),
            206 => Some("Partial Content".to_string()),
            207 => Some("Multi_Status".to_string()),
            208 => Some("Already Reported".to_string()),
            226 => Some("IM Used".to_string()),

            // Redirection Messages
            300 => Some("Multiple Choices".to_string()),
            301 => Some("Moved permanently".to_string()),
            302 => Some("Found".to_string()),
            303 => Some("See Other".to_string()),
            304 => Some("Not Modified".to_string()),
            305 => Some("Use Proxy".to_string()),  // Depreciated
            306 => Some("unused".to_string()), // Depreciated
            307 => Some("Temporary Redirect".to_string()), // No longer used, just reserved
            308 => Some("Permanent Redirect".to_string()),

            // Client error responses
            400 => Some("Bad Request".to_string()),
            401 => Some("Unauthorized".to_string()), // Experimental. Expect behaviour to change in the future.
            402 => Some("Payment Required".to_string()),
            403 => Some("Forbidden".to_string()),
            404 => Some("Not Found".to_string()),
            405 => Some("Method Not Allowed".to_string()),
            406 => Some("Not Acceptable".to_string()),
            407 => Some("Proxy Authentication Required".to_string()),
            408 => Some("Request Timeout".to_string()),
            409 => Some("Conflict".to_string()),
            410 => Some("Gone".to_string()),
            411 => Some("Length Required".to_string()),
            412 => Some("Precondition Failed".to_string()),
            413 => Some("Payload Too Large".to_string()),
            414 => Some("URI Too Long".to_string()),
            415 => Some("Unsupported Media Type".to_string()),
            416 => Some("Range Not Satisfiable".to_string()),
            417 => Some("Expectation Failed".to_string()),
            418 => Some("I',m a teapot".to_string()),
            421 => Some("Misdirected Request".to_string()),
            422 => Some("Unprocessable Content".to_string()),
            423 => Some("Locked".to_string()),
            424 => Some("Failed Dependency".to_string()), // Experimental. Expect behaviour to change in the future
            425 => Some("Too Early".to_string()),
            426 => Some("Upgrade Required".to_string()),
            428 => Some("Precondition Required".to_string()),
            429 => Some("Too Many Requests".to_string()),
            431 => Some("Request Header Fields Too Large".to_string()),
            451 => Some("Unavailable For Legal Reasons".to_string()),

            // Server error responses
            500 => Some("Internal Server Error".to_string()),
            501 => Some("Not Implemented".to_string()),
            502 => Some("Bad Gateway".to_string()),
            503 => Some("Service Unavailable".to_string()),
            504 => Some("Gateway Timeout".to_string()),
            505 => Some("HTTP Version Not Supported".to_string()),
            506 => Some("Variant Also Negotiates".to_string()),
            507 => Some("Insufficient Storage".to_string()),
            508 => Some("Loop Detected".to_string()),
            510 => Some("Not Extended".to_string()),
            511 => Some("Network Authentication Required".to_string()),
            _ => None
        };
    }
}


pub trait StatusCode {
    fn to_usize(&self) -> usize;
}

impl StatusCode for Status {
    fn to_usize(&self) -> usize {
        let status_code = StatusMethods::status_code(self);
        return status_code.unwrap();
    }
}

impl StatusCode for usize {
    fn to_usize(&self) -> usize {
        *self
    }
}