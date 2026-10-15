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

/** A place the user can pick: name, coordinates and the civil zone that applies there. */
data class Place(val name: String, val latitude: Double, val longitude: Double, val zoneId: String)

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
) {
    fun zone(): ZoneId = try { ZoneId.of(zoneId) } catch (e: DateTimeException) { ZoneId.systemDefault() }

    /** The stated civil time, resolved in the stated zone. */
    fun birthLocal(): ZonedDateTime =
        ZonedDateTime.of(LocalDateTime.of(year, month, day, hour, minute, second), zone())

    /** Local civil time in the profile's zone → UTC → exact instant. */
    fun birthInstant(): ExactInstant = Instants.fromInstant(birthLocal().toInstant())

    /** Half-window in seconds for the stated uncertainty level. */
    fun uncertaintySeconds(): Long = Uncertainty.seconds(uncertaintyLevel)

    fun place(): Place = Place(placeName, latitude, longitude, zoneId)
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
    private const val K_AYANAMSA = "settings.ayanamsa"
    private const val K_HOUSES = "settings.houses"
    private const val K_TOPO = "settings.topocentricMoon"
    private const val K_CUR_NAME = "current.name"
    private const val K_CUR_LAT = "current.lat"
    private const val K_CUR_LON = "current.lon"
    private const val K_CUR_ZONE = "current.zone"

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
            .apply()
    }

    fun settings(context: Context): EngineSettings {
        val p = prefs(context)
        val ayanamsa = runCatching { AyanamsaChoice.valueOf(p.getString(K_AYANAMSA, null) ?: "") }
            .getOrDefault(AyanamsaChoice.LAHIRI)
        val houses = runCatching { HouseSystem.valueOf(p.getString(K_HOUSES, null) ?: "") }
            .getOrDefault(HouseSystem.PLACIDUS)
        return EngineSettings(ayanamsa, houses, p.getBoolean(K_TOPO, false))
    }

    fun saveSettings(context: Context, s: EngineSettings) {
        prefs(context).edit()
            .putString(K_AYANAMSA, s.ayanamsa.name)
            .putString(K_HOUSES, s.houseSystem.name)
            .putBoolean(K_TOPO, s.topocentricMoon)
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
            .apply()
    }

    /** Major Indian and world cities for the picker. */
    val places: List<Place> = listOf(
        kochi,
        Place("Thiruvananthapuram", 8.5241, 76.9366, "Asia/Kolkata"),
        Place("Kozhikode", 11.2588, 75.7804, "Asia/Kolkata"),
        Place("Thrissur", 10.5276, 76.2144, "Asia/Kolkata"),
        Place("Chennai", 13.0827, 80.2707, "Asia/Kolkata"),
        Place("Bengaluru", 12.9716, 77.5946, "Asia/Kolkata"),
        Place("Hyderabad", 17.3850, 78.4867, "Asia/Kolkata"),
        Place("Mumbai", 19.0760, 72.8777, "Asia/Kolkata"),
        Place("Pune", 18.5204, 73.8567, "Asia/Kolkata"),
        Place("Delhi", 28.6139, 77.2090, "Asia/Kolkata"),
        Place("Kolkata", 22.5726, 88.3639, "Asia/Kolkata"),
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
