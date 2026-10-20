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
            val address = o.optJSONObject("address")
            val cc = address?.optString("country_code")?.uppercase().orEmpty()
            // Nominatim's ISO 3166-2 codes ("US-AZ", "CA-NS", "AU-QLD", "ES-CN" …) under the keys
            // ISO3166-2-lvl3 … lvl8: the state or province decides the zone in many countries.
            val subdivisions = ArrayList<String>()
            if (address != null) {
                val keys = address.keys()
                while (keys.hasNext()) {
                    val k = keys.next()
                    if (k.startsWith("ISO3166-2")) {
                        val v = address.optString(k).trim().uppercase()
                        if (v.isNotEmpty()) subdivisions.add(v)
                    }
                }
            }
            out.add(Place(name.ifEmpty { "%.2f, %.2f".format(lat, lon) }, lat, lon, com.kshana.app.data.CityClocks.defaultZone(zoneFor(cc, lon, lat, subdivisions), lat, lon)))
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
     * Derive a civil zone from a country code, the state or province (ISO 3166-2) where known,
     * and the longitude. The subdivision decides first; otherwise multi-zone countries are split
     * by longitude band, other countries use their primary zone or the device's zone list for
     * the country, and only then a fixed Etc/GMT offset (sign inverted: Etc/GMT+5 means UTC−5).
     * Every id is validated with ZoneId.of before it is returned.
     */
    fun zoneFor(countryCode: String, lon: Double, lat: Double = 0.0, subdivisions: List<String> = emptyList()): String {
        val cc = countryCode.trim().uppercase()
        for (code in subdivisions) {
            val z = subdivisionZone(code, lon, lat) ?: continue
            val valid = runCatching { ZoneId.of(z).id }.getOrNull()
            if (valid != null) return valid
        }
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
            else -> PRIMARY[cc] ?: icuZone(cc, lon) ?: etcZone(lon)
        }
        return runCatching { ZoneId.of(candidate).id }.getOrElse { etcZone(lon) }
    }

    /**
     * The zone of a state or province where a longitude band gets it wrong. A band cannot see
     * that Arizona, Saskatchewan, Queensland and the Northern Territory keep no summer time,
     * that Halifax and St John's are one and one and a half hours ahead of Toronto, or that
     * the Canary Islands are an hour behind Madrid. A wrong zone moves the birth by an hour
     * (or thirty minutes), and with it the rising sign and every dasha date, so the
     * subdivision decides wherever Nominatim gives it.
     */
    internal fun subdivisionZone(code: String, lon: Double, lat: Double): String? = when (code) {
        // United States
        "US-AZ" -> "America/Phoenix"
        "US-HI" -> "Pacific/Honolulu"
        "US-AK" -> "America/Anchorage"
        "US-PR" -> "America/Puerto_Rico"
        "US-GU" -> "Pacific/Guam"
        "US-VI" -> "America/St_Thomas"
        "US-MI" -> "America/Detroit"
        "US-IN" -> if (lon < -86.9) "America/Chicago" else "America/Indiana/Indianapolis"
        "US-KY" -> if (lon > -86.0) "America/Kentucky/Louisville" else "America/Chicago"
        "US-TN" -> if (lon > -85.45) "America/New_York" else "America/Chicago"
        "US-FL" -> if (lon < -85.0) "America/Chicago" else "America/New_York"
        "US-CT", "US-DE", "US-DC", "US-GA", "US-ME", "US-MD", "US-MA", "US-NH", "US-NJ", "US-NY",
        "US-NC", "US-OH", "US-PA", "US-RI", "US-SC", "US-VT", "US-VA", "US-WV" -> "America/New_York"
        "US-AL", "US-AR", "US-IL", "US-IA", "US-LA", "US-MN", "US-MS", "US-MO", "US-OK", "US-WI" -> "America/Chicago"
        "US-TX" -> if (lon < -104.9) "America/Denver" else "America/Chicago"
        "US-KS" -> if (lon < -101.5) "America/Denver" else "America/Chicago"
        "US-NE" -> if (lon < -101.0) "America/Denver" else "America/Chicago"
        "US-SD" -> if (lon < -100.5) "America/Denver" else "America/Chicago"
        "US-ND" -> if (lon < -101.3) "America/Denver" else "America/Chicago"
        "US-CO", "US-MT", "US-NM", "US-UT", "US-WY" -> "America/Denver"
        "US-ID" -> if (lat > 45.6) "America/Los_Angeles" else "America/Boise"
        "US-CA", "US-WA", "US-NV", "US-OR" -> "America/Los_Angeles"
        // Canada
        "CA-NL" -> if (lat > 51.5 && lon < -56.0) "America/Goose_Bay" else "America/St_Johns"
        "CA-NS", "CA-PE" -> "America/Halifax"
        "CA-NB" -> "America/Moncton"
        "CA-QC" -> "America/Toronto"
        "CA-ON" -> if (lon < -90.0) "America/Winnipeg" else "America/Toronto"
        "CA-MB" -> "America/Winnipeg"
        "CA-SK" -> "America/Regina"
        "CA-AB" -> "America/Edmonton"
        "CA-BC" -> if (lon > -117.5 && lat < 50.5) "America/Edmonton" else "America/Vancouver"
        "CA-YT" -> "America/Whitehorse"
        "CA-NT" -> "America/Yellowknife"
        "CA-NU" -> when {
            lon > -85.0 -> "America/Iqaluit"
            lon > -102.0 -> "America/Rankin_Inlet"
            else -> "America/Cambridge_Bay"
        }
        // Australia
        "AU-NSW" -> if (lon < 141.6) "Australia/Broken_Hill" else "Australia/Sydney"
        "AU-ACT" -> "Australia/Sydney"
        "AU-VIC" -> "Australia/Melbourne"
        "AU-QLD" -> "Australia/Brisbane"
        "AU-SA" -> "Australia/Adelaide"
        "AU-WA" -> "Australia/Perth"
        "AU-TAS" -> "Australia/Hobart"
        "AU-NT" -> "Australia/Darwin"
        // Brazil: summer time was kept by some states and not others
        "BR-SP", "BR-RJ", "BR-MG", "BR-ES", "BR-PR", "BR-SC", "BR-RS", "BR-GO", "BR-DF" -> "America/Sao_Paulo"
        "BR-BA" -> "America/Bahia"
        "BR-PE", "BR-PB", "BR-RN" -> "America/Recife"
        "BR-CE", "BR-PI", "BR-MA" -> "America/Fortaleza"
        "BR-AL", "BR-SE" -> "America/Maceio"
        "BR-PA" -> if (lon < -54.0) "America/Santarem" else "America/Belem"
        "BR-AP" -> "America/Belem"
        "BR-TO" -> "America/Araguaina"
        "BR-AM" -> if (lon < -67.5) "America/Eirunepe" else "America/Manaus"
        "BR-MT" -> "America/Cuiaba"
        "BR-MS" -> "America/Campo_Grande"
        "BR-RO" -> "America/Porto_Velho"
        "BR-RR" -> "America/Boa_Vista"
        "BR-AC" -> "America/Rio_Branco"
        // Mexico
        "MX-BCN" -> "America/Tijuana"
        "MX-BCS", "MX-SIN", "MX-NAY" -> "America/Mazatlan"
        "MX-SON" -> "America/Hermosillo"
        "MX-CHH" -> "America/Chihuahua"
        "MX-ROO" -> "America/Cancun"
        "MX-NLE", "MX-COA", "MX-DUR", "MX-TAM" -> "America/Monterrey"
        "MX-YUC", "MX-CAM" -> "America/Merida"
        // Islands an hour or more off their mainland
        "ES-CN" -> "Atlantic/Canary"
        "PT-20" -> "Atlantic/Azores"
        "PT-30" -> "Atlantic/Madeira"
        "EC-W" -> "Pacific/Galapagos"
        else -> null
    }

    /**
     * For a country with no rule here, the zones the device's own time-zone data lists for it
     * (ICU, Android 7+), choosing the one whose present standard offset is nearest the
     * longitude's mean-solar offset. A real zone keeps the country's summer-time and historical
     * rules, which a fixed Etc/GMT offset would drop.
     */
    private fun icuZone(cc: String, lon: Double): String? {
        if (cc.length != 2) return null
        val ids: Array<String> = runCatching { android.icu.util.TimeZone.getAvailableIDs(cc) }.getOrNull() ?: return null
        val now = java.time.Instant.now()
        var best: String? = null
        var bestGap = Double.MAX_VALUE
        for (id in ids) {
            if (id.startsWith("Etc/") || !id.contains('/')) continue
            val zone = runCatching { ZoneId.of(id) }.getOrNull() ?: continue
            val gap = kotlin.math.abs(zone.rules.getStandardOffset(now).totalSeconds / 3600.0 - lon / 15.0)
            if (gap < bestGap) {
                bestGap = gap
                best = zone.id
            }
        }
        return best
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
