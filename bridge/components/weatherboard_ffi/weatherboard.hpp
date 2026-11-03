#ifndef WEATHERBOARD_H
#define WEATHERBOARD_H

#include <cstdarg>
#include <cstddef>
#include <cstdint>
#include <cstdlib>
#include <ostream>
#include <new>

constexpr uint8_t WB_MAX_PAYLOAD_LEN = 255;

namespace wb {

enum class topic_t : uint8_t {
  WEATHER,
};

enum class location_t : uint8_t {
  LISBON,
  CAPARICA,
};

enum class day_t : uint8_t {
  SUNDAY,
  MONDAY,
  TUESDAY,
  WEDNESDAY,
  THURSDAY,
  FRIDAY,
  SATURDAY,
};

enum class month_t : uint8_t {
  JANUARY,
  FEBRUARY,
  MARCH,
  APRIL,
  MAY,
  JUNE,
  JULY,
  AUGUST,
  SEPTEMBER,
  OCTOBER,
  NOVEMBER,
  DECEMBER,
};

enum class condition_t : uint8_t {
  SUNNY,
  PARTLY_CLOUDY,
  CLOUDY,
  FOG,
  RAIN,
  HEAVY_RAIN,
  SNOW,
  THUNDERSTORM,
  WIND,
};

struct uuid128_t {
  uint8_t bytes[16];
};

struct day_forecast_t {
  condition_t condition;
  int8_t low_c;
  int8_t high_c;
};

struct weather_t {
  location_t location;
  day_t week_day;
  month_t month;
  uint8_t day;
  uint8_t updated[2];
  condition_t condition;
  int8_t temp_c;
  int8_t feels_like_c;
  uint8_t sunrise[2];
  uint8_t sunset[2];
  uint8_t wind_kmh;
  uint8_t humidity_pct;
  uint16_t pressure_hpa;
  uint8_t uvi_tenths;
  int8_t dew_point_c;
  uint8_t rain_pct;
  int8_t chart_low_c;
  int8_t chart_high_c;
  uint8_t chart_start_hour;
  int8_t chart_temps_c[12];
  uint8_t chart_pop_pct[12];
  day_forecast_t forecast[6];
};

extern "C" {

const char *topic_str(topic_t topic);

uuid128_t display_service_uuid();

uuid128_t weather_payload_uuid();

uuid128_t display_service_uuid_le();

uuid128_t weather_payload_uuid_le();

bool decode_weather(const uint8_t *buf, size_t len, weather_t *out);

}  // extern "C"

}  // namespace wb

#endif  // WEATHERBOARD_H
