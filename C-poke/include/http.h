#ifndef HTTP_H
#define HTTP_H

#include <stddef.h>

#define MAX_METHOD_LEN 8
#define MAX_PATH_LEN 256
#define MAX_HEADERS 32
#define MAX_HEADER_VALUE_LEN 512
#define MAX_BODY_LEN 4096
#define MAX_RESPONSE_LEN 16384

typedef struct {
    char key[64];
    char value[MAX_HEADER_VALUE_LEN];
} HttpHeader;

typedef struct {
    char method[MAX_METHOD_LEN];
    char path[MAX_PATH_LEN];
    HttpHeader headers[MAX_HEADERS];
    size_t header_count;
    char body[MAX_BODY_LEN];
    size_t body_len;
} HttpRequest;

typedef struct {
    int status_code;
    char status_text[32];
    HttpHeader headers[MAX_HEADERS];
    size_t header_count;
    char body[MAX_RESPONSE_LEN];
    size_t body_len;
} HttpResponse;

int http_request_parse(HttpRequest *req, const char *raw, size_t len);
void http_response_init(HttpResponse *res, int status_code, const char *status_text);
void http_response_set_header(HttpResponse *res, const char *key, const char *value);
void http_response_set_body(HttpResponse *res, const char *body);
int http_response_serialize(HttpResponse *res, char *buffer, size_t buffer_size);

const char *http_status_text(int status_code);

#endif
