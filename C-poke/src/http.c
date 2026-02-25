#include "http.h"
#include <stdio.h>
#include <string.h>
#include <ctype.h>

const char *http_status_text(int status_code) {
    switch (status_code) {
        case 200: return "OK";
        case 201: return "Created";
        case 204: return "No Content";
        case 400: return "Bad Request";
        case 404: return "Not Found";
        case 405: return "Method Not Allowed";
        case 413: return "Payload Too Large";
        case 500: return "Internal Server Error";
        default: return "Unknown";
    }
}

static int parse_request_line(HttpRequest *req, char *line) {
    char *saveptr;
    char *method = strtok_r(line, " ", &saveptr);
    char *path = strtok_r(NULL, " ", &saveptr);
    char *version = strtok_r(NULL, "\r", &saveptr);
    
    if (!method || !path || !version) return -1;
    if (strncmp(version, "HTTP/", 5) != 0) return -1;
    
    strncpy(req->method, method, MAX_METHOD_LEN - 1);
    req->method[MAX_METHOD_LEN - 1] = '\0';
    strncpy(req->path, path, MAX_PATH_LEN - 1);
    req->path[MAX_PATH_LEN - 1] = '\0';
    
    return 0;
}

static int parse_header(HttpRequest *req, char *line) {
    if (req->header_count >= MAX_HEADERS) return -1;
    
    char *colon = strchr(line, ':');
    if (!colon) return -1;
    
    *colon = '\0';
    char *value = colon + 1;
    while (*value == ' ') value++;
    
    size_t value_len = strlen(value);
    while (value_len > 0 && (value[value_len - 1] == '\r' || value[value_len - 1] == '\n')) {
        value[--value_len] = '\0';
    }
    
    strncpy(req->headers[req->header_count].key, line, 63);
    req->headers[req->header_count].key[63] = '\0';
    strncpy(req->headers[req->header_count].value, value, MAX_HEADER_VALUE_LEN - 1);
    req->headers[req->header_count].value[MAX_HEADER_VALUE_LEN - 1] = '\0';
    req->header_count++;
    
    return 0;
}

int http_request_parse(HttpRequest *req, const char *raw, size_t len) {
    if (!req || !raw || len == 0) return -1;
    
    memset(req, 0, sizeof(HttpRequest));
    
    char buffer[MAX_BODY_LEN + MAX_PATH_LEN + 512];
    size_t copy_len = len < sizeof(buffer) - 1 ? len : sizeof(buffer) - 1;
    memcpy(buffer, raw, copy_len);
    buffer[copy_len] = '\0';
    
    char *saveptr;
    char *line = strtok_r(buffer, "\n", &saveptr);
    
    if (!line || parse_request_line(req, line) != 0) return -1;
    
    while ((line = strtok_r(NULL, "\n", &saveptr)) != NULL) {
        if (line[0] == '\r' || line[0] == '\0') {
            break;
        }
        if (parse_header(req, line) != 0) {
            continue;
        }
    }
    
    char *body_start = strstr(raw, "\r\n\r\n");
    if (body_start) {
        body_start += 4;
        size_t body_len = len - (body_start - raw);
        if (body_len > 0 && body_len < MAX_BODY_LEN) {
            memcpy(req->body, body_start, body_len);
            req->body[body_len] = '\0';
            req->body_len = body_len;
        }
    }
    
    return 0;
}

void http_response_init(HttpResponse *res, int status_code, const char *status_text) {
    memset(res, 0, sizeof(HttpResponse));
    res->status_code = status_code;
    strncpy(res->status_text, status_text, 31);
    res->status_text[31] = '\0';
}

void http_response_set_header(HttpResponse *res, const char *key, const char *value) {
    if (res->header_count >= MAX_HEADERS) return;
    strncpy(res->headers[res->header_count].key, key, 63);
    res->headers[res->header_count].key[63] = '\0';
    strncpy(res->headers[res->header_count].value, value, MAX_HEADER_VALUE_LEN - 1);
    res->headers[res->header_count].value[MAX_HEADER_VALUE_LEN - 1] = '\0';
    res->header_count++;
}

void http_response_set_body(HttpResponse *res, const char *body) {
    size_t len = strlen(body);
    if (len >= MAX_RESPONSE_LEN) len = MAX_RESPONSE_LEN - 1;
    memcpy(res->body, body, len);
    res->body[len] = '\0';
    res->body_len = len;
}

int http_response_serialize(HttpResponse *res, char *buffer, size_t buffer_size) {
    int written = snprintf(buffer, buffer_size, 
        "HTTP/1.1 %d %s\r\n"
        "Server: C-Poke/1.0\r\n",
        res->status_code, res->status_text);
    
    if (written < 0 || (size_t)written >= buffer_size) return -1;
    
    for (size_t i = 0; i < res->header_count; i++) {
        int hw = snprintf(buffer + written, buffer_size - written,
            "%s: %s\r\n", res->headers[i].key, res->headers[i].value);
        if (hw < 0 || (size_t)(written + hw) >= buffer_size) return -1;
        written += hw;
    }
    
    if (res->body_len > 0) {
        int cw = snprintf(buffer + written, buffer_size - written,
            "Content-Length: %zu\r\n\r\n", res->body_len);
        if (cw < 0 || (size_t)(written + cw) >= buffer_size) return -1;
        written += cw;
        
        if (written + res->body_len >= buffer_size) return -1;
        memcpy(buffer + written, res->body, res->body_len);
        written += res->body_len;
    } else {
        int ew = snprintf(buffer + written, buffer_size - written, "\r\n");
        if (ew < 0 || (size_t)(written + ew) >= buffer_size) return -1;
        written += ew;
    }
    
    return written;
}
