#include "clarity/values.h"

#include <string.h>
#include <time.h>

static bool number(const char* text, size_t at, size_t length, int* out) {
    *out = 0;
    for (size_t i = at; i < at + length; ++i) {
        if (text[i] < '0' || text[i] > '9') return false;
        *out = *out * 10 + (text[i] - '0');
    }
    return true;
}

static bool leap(int year) { return (year % 4 == 0 && year % 100 != 0) || year % 400 == 0; }

static bool real(ClarityDate d) {
    static const int days[] = {31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31};
    if (d.year < 1 || d.month < 1 || d.month > 12 || d.day < 1) return false;
    int most = days[d.month - 1] + (d.month == 2 && leap(d.year) ? 1 : 0);
    return d.day <= most;
}

bool clarity_date_parse(const char* text, ClarityDate* out) {
    if (!text || strlen(text) != 10 || text[4] != '-' || text[7] != '-') return false;
    ClarityDate d;
    if (!number(text, 0, 4, &d.year) || !number(text, 5, 2, &d.month) || !number(text, 8, 2, &d.day)) return false;
    if (!real(d)) return false;
    *out = d;
    return true;
}

bool clarity_date_time_parse(const char* text, ClarityDateTime* out) {
    size_t length = text ? strlen(text) : 0;
    if (length < 16 || (text[10] != 'T' && text[10] != ' ') || text[13] != ':') return false;
    char day[11];
    memcpy(day, text, 10);
    day[10] = '\0';
    ClarityDate date;
    if (!clarity_date_parse(day, &date)) return false;
    ClarityDateTime t = {date.year, date.month, date.day, 0, 0, 0};
    if (!number(text, 11, 2, &t.hour) || !number(text, 14, 2, &t.minute)) return false;
    if (length >= 19 && text[16] == ':') {
        if (length != 19 || !number(text, 17, 2, &t.second)) return false;
    } else if (length != 16) {
        return false;
    }
    if (t.hour > 23 || t.minute > 59 || t.second > 59) return false;
    *out = t;
    return true;
}

const char* clarity_date_text(ClarityDate value) {
    if (clarity_date_is_none(value)) return "";
    return clarity_format("%04d-%02d-%02d", value.year, value.month, value.day);
}

const char* clarity_date_time_text(ClarityDateTime value) {
    if (clarity_date_time_is_none(value)) return "";
    return clarity_format("%04d-%02d-%02dT%02d:%02d:%02d", value.year, value.month, value.day, value.hour, value.minute, value.second);
}

bool clarity_date_is_none(ClarityDate value) { return value.year == 0; }
bool clarity_date_time_is_none(ClarityDateTime value) { return value.year == 0; }

static int order(int a, int b) { return a < b ? -1 : a > b ? 1 : 0; }

int clarity_date_compare(ClarityDate a, ClarityDate b) {
    if (clarity_date_is_none(a) || clarity_date_is_none(b)) return order(!clarity_date_is_none(a), !clarity_date_is_none(b));
    int c = order(a.year, b.year);
    if (!c) c = order(a.month, b.month);
    if (!c) c = order(a.day, b.day);
    return c;
}

int clarity_date_time_compare(ClarityDateTime a, ClarityDateTime b) {
    if (clarity_date_time_is_none(a) || clarity_date_time_is_none(b)) return order(!clarity_date_time_is_none(a), !clarity_date_time_is_none(b));
    int c = clarity_date_compare((ClarityDate){a.year, a.month, a.day}, (ClarityDate){b.year, b.month, b.day});
    if (!c) c = order(a.hour, b.hour);
    if (!c) c = order(a.minute, b.minute);
    if (!c) c = order(a.second, b.second);
    return c;
}

static struct tm local_now(void) {
    time_t t = time(NULL);
    struct tm out;
#ifdef _WIN32
    localtime_s(&out, &t);
#else
    localtime_r(&t, &out);
#endif
    return out;
}

ClarityDate clarity_today(void) {
    struct tm t = local_now();
    return (ClarityDate){t.tm_year + 1900, t.tm_mon + 1, t.tm_mday};
}

ClarityDateTime clarity_now(void) {
    struct tm t = local_now();
    return (ClarityDateTime){t.tm_year + 1900, t.tm_mon + 1, t.tm_mday, t.tm_hour, t.tm_min, t.tm_sec};
}
