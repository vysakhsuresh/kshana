package com.kshana.app.net

import android.content.Context
import com.kshana.app.data.Place
import kotlinx.coroutines.delay
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import org.json.JSONArray
import java.net.URLEncoder
import java.time.ZoneId
import kotlin.math.roundToInt

/**
 * Any place on Earth, by name, through OpenStreetMap's Nominatim. Nominatim asks for at most
 * one request per second and an identifying User-Agent; both are honoured here. It returns
 * no time zone, so one is derived from the country and the longitude.
 */
object PlaceSearch {
    private const val ENDPOINT = "https://nominatim.openstreetmap.org/search"
    private const val MIN_GAP_MS = 1_100L

    private val gate = Mutex()
    private var lastCallAt = 0L

    suspend fun searchPlaces(context: Context, query: String): List<Place> {
        Online.require(context)
        val q = query.trim()
        if (q.isEmpty()) return emptyList()
        val url = ENDPOINT +
            "?q=" + URLEncoder.encode(q, "UTF-8") +
            "&format=jsonv2&limit=8&addressdetails=1"
        val body = gate.withLock {
            val wait = lastCallAt + MIN_GAP_MS - System.currentTimeMillis()
            if (wait > 0) delay(wait)
            try {
                Online.getText(url, mapOf("Accept" to "application/json", "Accept-Language" to "en"))
            } finally {
                lastCallAt = System.currentTimeMillis()
            }
        }
        return parse(body)
    }

    internal fun parse(body: String): List<Place> {
        val arr = JSONArray(body)
        val out = ArrayList<Place>(arr.length())
        for (i in 0 until arr.length()) {
            val o = arr.optJSONObject(i) ?: continue
            val lat = o.optString("lat").toDoubleOrNull() ?: continue
            val lon = o.optString("lon").toDoubleOrNull() ?: continue
            val display = o.optString("display_name")
            val name = display.split(",").map { it.trim() }.filter { it.isNotEmpty() }.take(2).joinToString(", ")
            val cc = o.optJSONObject("address")?.optString("country_code")?.uppercase().orEmpty()
            out.add(Place(name.ifEmpty { "%.2f, %.2f".format(lat, lon) }, lat, lon, zoneFor(cc, lon)))
        }
        return out
    }

    /** Primary IANA zone for countries that have one civil zone (or one that nearly everyone uses). */
    private val PRIMARY: Map<String, String> = mapOf(
        "IN" to "Asia/Kolkata", "LK" to "Asia/Colombo", "NP" to "Asia/Kathmandu", "BD" to "Asia/Dhaka",
        "PK" to "Asia/Karachi", "AF" to "Asia/Kabul", "MM" to "Asia/Yangon", "BT" to "Asia/Thimphu",
        "MV" to "Indian/Maldives", "AE" to "Asia/Dubai", "SG" to "Asia/Singapore", "MY" to "Asia/Kuala_Lumpur",
        "GB" to "Europe/London", "DE" to "Europe/Berlin", "FR" to "Europe/Paris", "IT" to "Europe/Rome",
        "ES" to "Europe/Madrid", "NL" to "Europe/Amsterdam", "JP" to "Asia/Tokyo", "KR" to "Asia/Seoul",
        "CN" to "Asia/Shanghai", "HK" to "Asia/Hong_Kong", "TW" to "Asia/Taipei", "MO" to "Asia/Macau",
        "TH" to "Asia/Bangkok", "PH" to "Asia/Manila", "VN" to "Asia/Ho_Chi_Minh", "KH" to "Asia/Phnom_Penh",
        "LA" to "Asia/Vientiane", "SA" to "Asia/Riyadh", "QA" to "Asia/Qatar", "KW" to "Asia/Kuwait",
        "OM" to "Asia/Muscat", "BH" to "Asia/Bahrain", "IR" to "Asia/Tehran", "IQ" to "Asia/Baghdad",
        "IL" to "Asia/Jerusalem", "JO" to "Asia/Amman", "LB" to "Asia/Beirut", "SY" to "Asia/Damascus",
        "TR" to "Europe/Istanbul", "EG" to "Africa/Cairo", "ZA" to "Africa/Johannesburg", "KE" to "Africa/Nairobi",
        "NG" to "Africa/Lagos", "GH" to "Africa/Accra", "ET" to "Africa/Addis_Ababa", "TZ" to "Africa/Dar_es_Salaam",
        "UG" to "Africa/Kampala", "MA" to "Africa/Casablanca", "DZ" to "Africa/Algiers", "TN" to "Africa/Tunis",
        "MU" to "Indian/Mauritius", "NZ" to "Pacific/Auckland", "FJ" to "Pacific/Fiji",
        "AR" to "America/Argentina/Buenos_Aires", "CL" to "America/Santiago", "CO" to "America/Bogota",
        "PE" to "America/Lima", "VE" to "America/Caracas", "EC" to "America/Guayaquil", "BO" to "America/La_Paz",
        "PY" to "America/Asuncion", "UY" to "America/Montevideo", "CU" to "America/Havana", "JM" to "America/Jamaica",
        "GT" to "America/Guatemala", "CR" to "America/Costa_Rica", "PA" to "America/Panama",
        "IE" to "Europe/Dublin", "PT" to "Europe/Lisbon", "BE" to "Europe/Brussels", "CH" to "Europe/Zurich",
        "AT" to "Europe/Vienna", "SE" to "Europe/Stockholm", "NO" to "Europe/Oslo", "DK" to "Europe/Copenhagen",
        "FI" to "Europe/Helsinki", "PL" to "Europe/Warsaw", "CZ" to "Europe/Prague", "SK" to "Europe/Bratislava",
        "GR" to "Europe/Athens", "UA" to "Europe/Kyiv", "RO" to "Europe/Bucharest", "HU" to "Europe/Budapest",
        "BG" to "Europe/Sofia", "RS" to "Europe/Belgrade", "HR" to "Europe/Zagreb", "SI" to "Europe/Ljubljana",
        "BA" to "Europe/Sarajevo", "LT" to "Europe/Vilnius", "LV" to "Europe/Riga", "EE" to "Europe/Tallinn",
        "BY" to "Europe/Minsk", "MD" to "Europe/Chisinau", "IS" to "Atlantic/Reykjavik", "LU" to "Europe/Luxembourg",
        "MT" to "Europe/Malta", "CY" to "Asia/Nicosia", "GE" to "Asia/Tbilisi", "AM" to "Asia/Yerevan",
        "AZ" to "Asia/Baku", "UZ" to "Asia/Tashkent", "KZ" to "Asia/Almaty", "KG" to "Asia/Bishkek",
        "TJ" to "Asia/Dushanbe", "TM" to "Asia/Ashgabat", "MN" to "Asia/Ulaanbaatar", "KP" to "Asia/Pyongyang",
    )

    /**
     * Derive a civil zone from a country code and longitude. Multi-zone countries are split by
     * longitude band; unknown countries fall back to a fixed Etc/GMT offset (sign inverted:
     * Etc/GMT+5 means UTC−5). Every id is validated with ZoneId.of before it is returned.
     */
    fun zoneFor(countryCode: String, lon: Double): String {
        val cc = countryCode.trim().uppercase()
        val candidate = when (cc) {
            "US" -> when {
                lon > -85.0 -> "America/New_York"
                lon > -100.0 -> "America/Chicago"
                lon > -115.0 -> "America/Denver"
                lon > -140.0 -> "America/Los_Angeles"
                lon > -160.0 -> "America/Anchorage"
                else -> "Pacific/Honolulu"
            }
            "CA" -> when {
                lon > -90.0 -> "America/Toronto"
                lon > -102.0 -> "America/Winnipeg"
                lon > -120.0 -> "America/Edmonton"
                else -> "America/Vancouver"
            }
            "AU" -> when {
                lon > 141.0 -> "Australia/Sydney"
                lon > 129.0 -> "Australia/Adelaide"
                else -> "Australia/Perth"
            }
            "BR" -> if (lon < -60.0) "America/Manaus" else "America/Sao_Paulo"
            "RU" -> when {
                lon < 40.0 -> "Europe/Moscow"
                lon < 60.0 -> "Asia/Yekaterinburg"
                lon < 90.0 -> "Asia/Novosibirsk"
                lon < 110.0 -> "Asia/Irkutsk"
                lon < 130.0 -> "Asia/Yakutsk"
                else -> "Asia/Vladivostok"
            }
            "MX" -> when {
                lon > -105.0 -> "America/Mexico_City"
                lon > -112.0 -> "America/Chihuahua"
                else -> "America/Tijuana"
            }
            "ID" -> when {
                lon < 112.0 -> "Asia/Jakarta"
                lon < 128.0 -> "Asia/Makassar"
                else -> "Asia/Jayapura"
            }
            else -> PRIMARY[cc] ?: etcZone(lon)
        }
        return runCatching { ZoneId.of(candidate).id }.getOrElse { etcZone(lon) }
    }

    /** Etc/GMT±n for a longitude; the sign is inverted by convention (Etc/GMT+5 is UTC−5). */
    private fun etcZone(lon: Double): String {
        val hours = (lon / 15.0).roundToInt().coerceIn(-12, 14)
        val id = when {
            hours == 0 -> "Etc/GMT"
            hours > 0 -> "Etc/GMT-$hours"
            else -> "Etc/GMT+${-hours}"
        }
        return runCatching { ZoneId.of(id).id }.getOrDefault("Etc/GMT")
    }
}

/** Convenience top-level entry point: `searchPlaces(context, "Kochi")`. */
suspend fun searchPlaces(context: Context, query: String): List<Place> = PlaceSearch.searchPlaces(context, query)
