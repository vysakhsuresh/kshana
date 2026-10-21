package com.kshana.app.data

import android.content.Context
import android.content.SharedPreferences
import com.kshana.app.engine.Instants
import uniffi.kshana_ffi.AyanamsaChoice
import uniffi.kshana_ffi.ExactInstant
import uniffi.kshana_ffi.HouseSystem
import java.time.DateTimeException
import java.time.LocalDateTime
import java.time.ZoneId
import java.time.ZonedDateTime

/**
 * A place the user can pick: name, coordinates, the civil zone that applies there, and its
 * height above sea level in metres (0 when unknown), which moves sunrise and sunset.
 */
data class Place(val name: String, val latitude: Double, val longitude: Double, val zoneId: String, val elevationM: Double = 0.0)

/**
 * The birth as the user states it: local civil time in the place's zone, a stated
 * uncertainty, and where. Everything stays on the phone.
 */
data class BirthProfile(
    val name: String,
    val year: Int,
    val month: Int,
    val day: Int,
    val hour: Int,
    val minute: Int,
    val second: Int,
    /** 0 exact · 1 ±1 min · 2 ±5 min · 3 ±15 min · 4 ±1 h · 5 ±3 h · 6 unknown. */
    val uncertaintyLevel: Int,
    val latitude: Double,
    val longitude: Double,
    val placeName: String,
    val zoneId: String,
    /** Height of the birth place above sea level, metres (0 when unknown). */
    val elevationM: Double = 0.0,
    /** The nazhika count the time was worked out from, when the horoscope gave one. */
    val nazhika: NazhikaEntry? = null,
) {
    /**
     * The place's zone for the present day and for dates after birth (events, transits). A
     * city-clock id ([CityClocks], e.g. "Kshana/Bombay") is Asia/Kolkata here.
     */
    fun zone(): ZoneId = try { CityClocks.of(zoneId) } catch (e: DateTimeException) { ZoneId.systemDefault() }

    /**
     * The zone the birth clock was read in: for a city-clock id before the city's switch to
     * IST, the city's own fixed offset (Bombay +04:51, Calcutta +05:53:20); otherwise [zone].
     * Use it for the birth moment and for times around it (birth-time candidates).
     */
    fun birthZone(): ZoneId {
        val local = runCatching { LocalDateTime.of(year, month, day, hour, minute, second) }.getOrNull()
        return CityClocks.zone(zoneId, local) ?: ZoneId.systemDefault()
    }

    /** The stated civil time, resolved in the zone of the birth clock. */
    fun birthLocal(): ZonedDateTime =
        ZonedDateTime.of(LocalDateTime.of(year, month, day, hour, minute, second), birthZone())

    /** Local civil time in the profile's zone → UTC → exact instant. */
    fun birthInstant(): ExactInstant = Instants.fromInstant(birthLocal().toInstant())

    /** Half-window in seconds for the stated uncertainty level. */
    fun uncertaintySeconds(): Long = Uncertainty.seconds(uncertaintyLevel)

    fun place(): Place = Place(placeName, latitude, longitude, zoneId, elevationM)
}

/**
 * A birth time as a Kerala or Tamil horoscope writes it: nazhika (24 minutes) and
 * vinazhika (24 seconds) counted from sunrise, or from six in the morning, on the date
 * the person typed. [previousDay]: born after midnight and before sunrise, so the count
 * starts at the previous morning's sunrise. [vinazhika] is null when the horoscope gives
 * whole nazhika only. Kept so the entry can be shown again; the clock time in the profile
 * is what every reading uses.
 */
data class NazhikaEntry(
    val year: Int,
    val month: Int,
    val day: Int,
    val nazhika: Int,
    val vinazhika: Int?,
    val fromSunrise: Boolean = true,
    val previousDay: Boolean = false,
) {
    /** "1990-2-20|25|12|sunrise|day": stored in preferences and in backups. */
    fun toText(): String =
        "$year-$month-$day|$nazhika|${vinazhika ?: ""}|${if (fromSunrise) "sunrise" else "six"}|${if (previousDay) "prev" else "day"}"

    /** "25 nazhika 12 vinazhika from sunrise". */
    fun label(): String =
        "$nazhika nazhika" + (vinazhika?.let { " $it vinazhika" } ?: "") + if (fromSunrise) " from sunrise" else " from 6 am"

    companion object {
        fun parse(s: String?): NazhikaEntry? {
            if (s.isNullOrBlank()) return null
            return runCatching {
                val p = s.split('|')
                val d = p[0].split('-').map { it.toInt() }
                NazhikaEntry(d[0], d[1], d[2], p[1].toInt(), p[2].toIntOrNull(), p.getOrNull(3) != "six", p.getOrNull(4) == "prev")
            }.getOrNull()
        }

        /** How sure a nazhika time is: within a minute with vinazhika, else within a quarter hour (a nazhika is 24 minutes). */
        fun uncertaintyLevel(vinazhikaGiven: Boolean): Int = if (vinazhikaGiven) 1 else 3
    }
}

/** The seven uncertainty levels with their labels and half-windows. */
object Uncertainty {
    val labels = listOf("exact", "±1 min", "±5 min", "±15 min", "±1 h", "±3 h", "unknown")
    private val seconds = longArrayOf(0L, 60L, 300L, 900L, 3600L, 10800L, 43200L)
    fun seconds(level: Int): Long = seconds[level.coerceIn(0, 6)]
    fun label(level: Int): String = labels[level.coerceIn(0, 6)]

    /** "±15 min" / "±1 h" / "±12 h" for a half-window, for the band line under a wheel. */
    fun windowText(level: Int): String {
        val s = seconds(level)
        return when {
            s == 0L -> "exact"
            s < 3600L -> "±${s / 60} min"
            else -> "±${s / 3600} h"
        }
    }
}

/** Engine choices the user can change in Settings. */
data class EngineSettings(
    val ayanamsa: AyanamsaChoice = AyanamsaChoice.LAHIRI,
    val houseSystem: HouseSystem = HouseSystem.PLACIDUS,
    val topocentricMoon: Boolean = false,
    /** "hindu" (Sun's centre on the true horizon) or "western" (refracted upper limb). */
    val riseConvention: String = "hindu",
    /** "true" (the osculating node, as the natal chart reckons it) or "mean": the point Rahu and Ketu stand at everywhere. */
    val lunarNode: String = "true",
)

/** SharedPreferences-backed store for the profile, engine settings and small per-screen state. */
object ProfileStore {
    private const val PREFS = "kshana"
    private const val K_HAS = "profile.has"
    private const val K_NAME = "profile.name"
    private const val K_YEAR = "profile.year"
    private const val K_MONTH = "profile.month"
    private const val K_DAY = "profile.day"
    private const val K_HOUR = "profile.hour"
    private const val K_MINUTE = "profile.minute"
    private const val K_SECOND = "profile.second"
    private const val K_UNC = "profile.uncertainty"
    private const val K_LAT = "profile.lat"
    private const val K_LON = "profile.lon"
    private const val K_PLACE = "profile.place"
    private const val K_ZONE = "profile.zone"
    private const val K_ELEV = "profile.elevation"
    private const val K_NAZHIKA = "profile.nazhika"
    private const val K_AYANAMSA = "settings.ayanamsa"
    private const val K_HOUSES = "settings.houses"
    private const val K_TOPO = "settings.topocentricMoon"
    private const val K_RISE = "settings.riseConvention"
    private const val K_NODE = "settings.lunarNode"
    private const val K_CUR_NAME = "current.name"
    private const val K_CUR_LAT = "current.lat"
    private const val K_CUR_LON = "current.lon"
    private const val K_CUR_ZONE = "current.zone"
    private const val K_CUR_ELEV = "current.elevation"

    val kochi = Place("Kochi", 9.9312, 76.2673, "Asia/Kolkata")

    /** Shared preferences for every small piece of state the screens keep (answers, commitments). */
    fun prefs(context: Context): SharedPreferences =
        context.applicationContext.getSharedPreferences(PREFS, Context.MODE_PRIVATE)

    fun load(context: Context): BirthProfile? {
        val p = prefs(context)
        if (!p.getBoolean(K_HAS, false)) return null
        return BirthProfile(
            name = p.getString(K_NAME, "") ?: "",
            year = p.getInt(K_YEAR, 2000),
            month = p.getInt(K_MONTH, 1),
            day = p.getInt(K_DAY, 1),
            hour = p.getInt(K_HOUR, 12),
            minute = p.getInt(K_MINUTE, 0),
            second = p.getInt(K_SECOND, 0),
            uncertaintyLevel = p.getInt(K_UNC, 0),
            latitude = p.getFloat(K_LAT, kochi.latitude.toFloat()).toDouble(),
            longitude = p.getFloat(K_LON, kochi.longitude.toFloat()).toDouble(),
            placeName = p.getString(K_PLACE, kochi.name) ?: kochi.name,
            zoneId = p.getString(K_ZONE, kochi.zoneId) ?: kochi.zoneId,
            elevationM = p.getFloat(K_ELEV, 0f).toDouble(),
            nazhika = NazhikaEntry.parse(p.getString(K_NAZHIKA, null)),
        )
    }

    fun save(context: Context, profile: BirthProfile) {
        prefs(context).edit()
            .putBoolean(K_HAS, true)
            .putString(K_NAME, profile.name)
            .putInt(K_YEAR, profile.year)
            .putInt(K_MONTH, profile.month)
            .putInt(K_DAY, profile.day)
            .putInt(K_HOUR, profile.hour)
            .putInt(K_MINUTE, profile.minute)
            .putInt(K_SECOND, profile.second)
            .putInt(K_UNC, profile.uncertaintyLevel)
            .putFloat(K_LAT, profile.latitude.toFloat())
            .putFloat(K_LON, profile.longitude.toFloat())
            .putString(K_PLACE, profile.placeName)
            .putString(K_ZONE, profile.zoneId)
            .putFloat(K_ELEV, profile.elevationM.toFloat())
            .apply { if (profile.nazhika != null) putString(K_NAZHIKA, profile.nazhika.toText()) else remove(K_NAZHIKA) }
            .apply()
        Heights.register(context)
    }

    fun settings(context: Context): EngineSettings {
        val p = prefs(context)
        val ayanamsa = runCatching { AyanamsaChoice.valueOf(p.getString(K_AYANAMSA, null) ?: "") }
            .getOrDefault(AyanamsaChoice.LAHIRI)
        val houses = runCatching { HouseSystem.valueOf(p.getString(K_HOUSES, null) ?: "") }
            .getOrDefault(HouseSystem.PLACIDUS)
        val rise = p.getString(K_RISE, null)?.takeIf { it == "hindu" || it == "western" } ?: "hindu"
        val node = p.getString(K_NODE, null)?.takeIf { it == "true" || it == "mean" } ?: "true"
        return EngineSettings(ayanamsa, houses, p.getBoolean(K_TOPO, false), rise, node)
    }

    fun saveSettings(context: Context, s: EngineSettings) {
        prefs(context).edit()
            .putString(K_AYANAMSA, s.ayanamsa.name)
            .putString(K_HOUSES, s.houseSystem.name)
            .putBoolean(K_TOPO, s.topocentricMoon)
            .putString(K_RISE, s.riseConvention)
            .putString(K_NODE, s.lunarNode)
            .apply()
    }

    /**
     * Where the phone is now: an explicitly saved current place, else the birth place,
     * else Kochi.
     */
    fun currentLocation(context: Context): Place {
        val p = prefs(context)
        val name = p.getString(K_CUR_NAME, null)
        if (name != null) {
            return Place(
                name,
                p.getFloat(K_CUR_LAT, kochi.latitude.toFloat()).toDouble(),
                p.getFloat(K_CUR_LON, kochi.longitude.toFloat()).toDouble(),
                p.getString(K_CUR_ZONE, kochi.zoneId) ?: kochi.zoneId,
                p.getFloat(K_CUR_ELEV, 0f).toDouble(),
            )
        }
        return load(context)?.place() ?: kochi
    }

    fun saveCurrentLocation(context: Context, place: Place) {
        prefs(context).edit()
            .putString(K_CUR_NAME, place.name)
            .putFloat(K_CUR_LAT, place.latitude.toFloat())
            .putFloat(K_CUR_LON, place.longitude.toFloat())
            .putString(K_CUR_ZONE, place.zoneId)
            .putFloat(K_CUR_ELEV, place.elevationM.toFloat())
            .apply()
        Heights.register(context)
    }

    // ----- Partners: other people's births, for Together. Stored as JSON on the phone. -----

    private const val K_PARTNERS = "partners.json"
    private const val K_RELATIONS = "partners.relations.json"

    /** The relations people can be given under Us. */
    val relations = listOf("Partner", "Spouse", "Mother", "Father", "Child", "Sibling", "Friend", "Other")

    /** The relation stored for a person's name, "Partner" when none. */
    fun relationOf(context: Context, name: String): String =
        runCatching { org.json.JSONObject(prefs(context).getString(K_RELATIONS, "{}") ?: "{}").optString(name, "Partner") }.getOrDefault("Partner")

    fun saveRelation(context: Context, name: String, relation: String) {
        val o = runCatching { org.json.JSONObject(prefs(context).getString(K_RELATIONS, "{}") ?: "{}") }.getOrDefault(org.json.JSONObject())
        o.put(name, relation)
        prefs(context).edit().putString(K_RELATIONS, o.toString()).apply()
    }
    private const val K_TOGETHER = "partners.together.json"

    /**
     * Whether the person and this partner already share a life. Then the match is read
     * as care for a bond that exists, never as a yes or no. Unset, a spouse or partner
     * counts as together.
     */
    fun isTogether(context: Context, name: String): Boolean {
        val o = runCatching { org.json.JSONObject(prefs(context).getString(K_TOGETHER, "{}") ?: "{}") }.getOrDefault(org.json.JSONObject())
        return if (o.has(name)) o.optBoolean(name) else relationOf(context, name) in setOf("Spouse", "Partner")
    }

    fun saveTogether(context: Context, name: String, together: Boolean) {
        val o = runCatching { org.json.JSONObject(prefs(context).getString(K_TOGETHER, "{}") ?: "{}") }.getOrDefault(org.json.JSONObject())
        o.put(name, together)
        prefs(context).edit().putString(K_TOGETHER, o.toString()).apply()
    }
    private const val K_PARTNER_SELECTED = "partners.selected"
    private const val K_SELF_GROOM = "partners.selfIsGroom"

    fun partners(context: Context): List<BirthProfile> {
        val raw = prefs(context).getString(K_PARTNERS, null) ?: return emptyList()
        return runCatching {
            val arr = org.json.JSONArray(raw)
            (0 until arr.length()).map { i ->
                val o = arr.getJSONObject(i)
                BirthProfile(
                    name = o.optString("name", "Partner"),
                    year = o.getInt("year"), month = o.getInt("month"), day = o.getInt("day"),
                    hour = o.getInt("hour"), minute = o.getInt("minute"), second = o.optInt("second", 0),
                    uncertaintyLevel = o.optInt("unc", 0),
                    latitude = o.getDouble("lat"), longitude = o.getDouble("lon"),
                    placeName = o.optString("place", ""), zoneId = o.optString("zone", "Asia/Kolkata"),
                    elevationM = o.optDouble("elev", 0.0).takeIf { it.isFinite() } ?: 0.0,
                    nazhika = NazhikaEntry.parse(o.optString("nazhika", "")),
                )
            }
        }.getOrDefault(emptyList())
    }

    fun savePartners(context: Context, list: List<BirthProfile>) {
        val arr = org.json.JSONArray()
        for (p in list) {
            arr.put(
                org.json.JSONObject()
                    .put("name", p.name).put("year", p.year).put("month", p.month).put("day", p.day)
                    .put("hour", p.hour).put("minute", p.minute).put("second", p.second).put("unc", p.uncertaintyLevel)
                    .put("lat", p.latitude).put("lon", p.longitude).put("place", p.placeName).put("zone", p.zoneId)
                    .put("elev", p.elevationM).apply { p.nazhika?.let { put("nazhika", it.toText()) } },
            )
        }
        prefs(context).edit().putString(K_PARTNERS, arr.toString()).apply()
        Heights.register(context)
    }

    /** Adds or replaces (by name) a partner and selects it. */
    fun addPartner(context: Context, p: BirthProfile) {
        val list = partners(context).filter { it.name != p.name } + p
        savePartners(context, list)
        selectPartner(context, list.lastIndex)
    }

    fun removePartner(context: Context, index: Int) {
        val list = partners(context).toMutableList()
        if (index in list.indices) list.removeAt(index)
        savePartners(context, list)
        selectPartner(context, (selectedPartner(context)).coerceAtMost(list.lastIndex).coerceAtLeast(0))
    }

    fun selectedPartner(context: Context): Int = prefs(context).getInt(K_PARTNER_SELECTED, 0)
    fun selectPartner(context: Context, index: Int) = prefs(context).edit().putInt(K_PARTNER_SELECTED, index).apply()

    /** Whether the one-directional kootas are counted with this person as the groom. */
    fun selfIsGroom(context: Context): Boolean = prefs(context).getBoolean(K_SELF_GROOM, true)
    fun saveSelfIsGroom(context: Context, v: Boolean) = prefs(context).edit().putBoolean(K_SELF_GROOM, v).apply()

    /** Major Indian and world cities for the picker. */
    val places: List<Place> = listOf(
        kochi,
        Place("Thiruvananthapuram", 8.5241, 76.9366, "Asia/Kolkata"),
        Place("Kozhikode", 11.2588, 75.7804, "Asia/Kolkata"),
        Place("Thrissur", 10.5276, 76.2144, "Asia/Kolkata"),
        Place("Chennai", 13.0827, 80.2707, "Asia/Kolkata"),
        Place("Bengaluru", 12.9716, 77.5946, "Asia/Kolkata"),
        Place("Hyderabad", 17.3850, 78.4867, "Asia/Kolkata"),
        Place("Mumbai", 19.0760, 72.8777, CityClocks.BOMBAY),
        Place("Pune", 18.5204, 73.8567, "Asia/Kolkata"),
        Place("Delhi", 28.6139, 77.2090, "Asia/Kolkata"),
        Place("Kolkata", 22.5726, 88.3639, CityClocks.CALCUTTA),
        Place("Ahmedabad", 23.0225, 72.5714, "Asia/Kolkata"),
        Place("Jaipur", 26.9124, 75.7873, "Asia/Kolkata"),
        Place("Lucknow", 26.8467, 80.9462, "Asia/Kolkata"),
        Place("Chandigarh", 30.7333, 76.7794, "Asia/Kolkata"),
        Place("Bhopal", 23.2599, 77.4126, "Asia/Kolkata"),
        Place("Coimbatore", 11.0168, 76.9558, "Asia/Kolkata"),
        Place("Madurai", 9.9252, 78.1198, "Asia/Kolkata"),
        Place("Visakhapatnam", 17.6868, 83.2185, "Asia/Kolkata"),
        Place("Guwahati", 26.1445, 91.7362, "Asia/Kolkata"),
        Place("Varanasi", 25.3176, 82.9739, "Asia/Kolkata"),
        Place("Colombo", 6.9271, 79.8612, "Asia/Colombo"),
        Place("Kathmandu", 27.7172, 85.3240, "Asia/Kathmandu"),
        Place("Dhaka", 23.8103, 90.4125, "Asia/Dhaka"),
        Place("Karachi", 24.8607, 67.0011, "Asia/Karachi"),
        Place("Dubai", 25.2048, 55.2708, "Asia/Dubai"),
        Place("Singapore", 1.3521, 103.8198, "Asia/Singapore"),
        Place("Kuala Lumpur", 3.1390, 101.6869, "Asia/Kuala_Lumpur"),
        Place("Hong Kong", 22.3193, 114.1694, "Asia/Hong_Kong"),
        Place("Tokyo", 35.6762, 139.6503, "Asia/Tokyo"),
        Place("Seoul", 37.5665, 126.9780, "Asia/Seoul"),
        Place("Sydney", -33.8688, 151.2093, "Australia/Sydney"),
        Place("Melbourne", -37.8136, 144.9631, "Australia/Melbourne"),
        Place("Auckland", -36.8509, 174.7645, "Pacific/Auckland"),
        Place("London", 51.5074, -0.1278, "Europe/London"),
        Place("Paris", 48.8566, 2.3522, "Europe/Paris"),
        Place("Berlin", 52.5200, 13.4050, "Europe/Berlin"),
        Place("Rome", 41.9028, 12.4964, "Europe/Rome"),
        Place("Madrid", 40.4168, -3.7038, "Europe/Madrid"),
        Place("Moscow", 55.7558, 37.6173, "Europe/Moscow"),
        Place("Istanbul", 41.0082, 28.9784, "Europe/Istanbul"),
        Place("Cairo", 30.0444, 31.2357, "Africa/Cairo"),
        Place("Nairobi", -1.2921, 36.8219, "Africa/Nairobi"),
        Place("Johannesburg", -26.2041, 28.0473, "Africa/Johannesburg"),
        Place("New York", 40.7128, -74.0060, "America/New_York"),
        Place("Toronto", 43.6532, -79.3832, "America/Toronto"),
        Place("Chicago", 41.8781, -87.6298, "America/Chicago"),
        Place("Los Angeles", 34.0522, -118.2437, "America/Los_Angeles"),
        Place("San Francisco", 37.7749, -122.4194, "America/Los_Angeles"),
        Place("Mexico City", 19.4326, -99.1332, "America/Mexico_City"),
        Place("São Paulo", -23.5505, -46.6333, "America/Sao_Paulo"),
        Place("Buenos Aires", -34.6037, -58.3816, "America/Argentina/Buenos_Aires"),
    )
}

/* ------------------------------------------------------------------------------------
 * Family profiles, the plain/astrologer mode, and online preferences. Everything stays
 * in the same SharedPreferences file; profiles are a JSON array so any number fit.
 * ---------------------------------------------------------------------------------- */

/** JSON round trip for a profile (org.json ships with Android). */
fun BirthProfile.toJson(): org.json.JSONObject = org.json.JSONObject()
    .put("name", name).put("year", year).put("month", month).put("day", day)
    .put("hour", hour).put("minute", minute).put("second", second)
    .put("uncertainty", uncertaintyLevel).put("lat", latitude).put("lon", longitude)
    .put("place", placeName).put("zone", zoneId).put("elev", elevationM)
    .apply { nazhika?.let { put("nazhika", it.toText()) } }

fun birthProfileFromJson(o: org.json.JSONObject): BirthProfile = BirthProfile(
    name = o.optString("name", ""),
    year = o.optInt("year", 2000), month = o.optInt("month", 1), day = o.optInt("day", 1),
    hour = o.optInt("hour", 12), minute = o.optInt("minute", 0), second = o.optInt("second", 0),
    uncertaintyLevel = o.optInt("uncertainty", 0),
    latitude = o.optDouble("lat", ProfileStore.kochi.latitude),
    longitude = o.optDouble("lon", ProfileStore.kochi.longitude),
    placeName = o.optString("place", ProfileStore.kochi.name),
    zoneId = o.optString("zone", ProfileStore.kochi.zoneId),
    // Files written before heights and nazhika entries were kept have neither: sea level, a clock time.
    elevationM = o.optDouble("elev", 0.0).takeIf { it.isFinite() } ?: 0.0,
    nazhika = NazhikaEntry.parse(o.optString("nazhika", "")),
)

/** The people the app knows besides the active profile: partner, children, parents. */
object FamilyStore {
    private const val K_FAMILY = "family.json"
    private const val K_MODE_SIMPLE = "ui.simple"
    private const val K_ONLINE = "online.allowed"
    private const val K_AI_KEY = "online.aiKey"
    private const val K_EOP_STAMP = "online.eopFetchedAt"
    private const val K_REMINDERS = "reminders.json"

    fun family(context: Context): List<BirthProfile> {
        val raw = ProfileStore.prefs(context).getString(K_FAMILY, null) ?: return emptyList()
        return runCatching {
            val arr = org.json.JSONArray(raw)
            (0 until arr.length()).map { birthProfileFromJson(arr.getJSONObject(it)) }
        }.getOrDefault(emptyList())
    }

    fun saveFamily(context: Context, list: List<BirthProfile>) {
        val arr = org.json.JSONArray()
        for (p in list) arr.put(p.toJson())
        ProfileStore.prefs(context).edit().putString(K_FAMILY, arr.toString()).apply()
        Heights.register(context)
    }

    /** Add or replace by name. */
    fun upsert(context: Context, p: BirthProfile) {
        val list = family(context).filter { it.name != p.name } + p
        saveFamily(context, list)
    }

    fun remove(context: Context, name: String) = saveFamily(context, family(context).filter { it.name != name })

    /** Everyone: the active profile first, then the family. */
    fun everyone(context: Context): List<BirthProfile> =
        listOfNotNull(ProfileStore.load(context)) + family(context)

    /** Plain words (true) or the full instrument (false). Default plain. */
    fun simpleMode(context: Context): Boolean = ProfileStore.prefs(context).getBoolean(K_MODE_SIMPLE, true)
    fun saveSimpleMode(context: Context, simple: Boolean) =
        ProfileStore.prefs(context).edit().putBoolean(K_MODE_SIMPLE, simple).apply()

    /** The user has allowed the app to use the network (place search, Earth-rotation data, narration). Default off. */
    fun onlineAllowed(context: Context): Boolean = ProfileStore.prefs(context).getBoolean(K_ONLINE, false)
    fun saveOnlineAllowed(context: Context, allowed: Boolean) =
        ProfileStore.prefs(context).edit().putBoolean(K_ONLINE, allowed).apply()

    /** A user-supplied Claude API key for optional narration. Stored only on the phone. */
    fun aiKey(context: Context): String? = ProfileStore.prefs(context).getString(K_AI_KEY, null)?.takeIf { it.isNotBlank() }
    fun saveAiKey(context: Context, key: String?) =
        ProfileStore.prefs(context).edit().putString(K_AI_KEY, key?.trim().orEmpty()).apply()

    /** Epoch millis of the last successful Earth-rotation (IERS finals) download, or 0. */
    fun eopFetchedAt(context: Context): Long = ProfileStore.prefs(context).getLong(K_EOP_STAMP, 0L)
    fun saveEopFetchedAt(context: Context, millis: Long) =
        ProfileStore.prefs(context).edit().putLong(K_EOP_STAMP, millis).apply()

    /** Scheduled reminders as JSON: [{"id":int,"atMillis":long,"title":str,"text":str}]. */
    fun remindersJson(context: Context): String = ProfileStore.prefs(context).getString(K_REMINDERS, "[]") ?: "[]"
    fun saveRemindersJson(context: Context, json: String) =
        ProfileStore.prefs(context).edit().putString(K_REMINDERS, json).apply()
}
