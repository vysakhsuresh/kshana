package com.kshana.app.ui.screens

import android.content.Context
import androidx.compose.ui.graphics.Color
import com.kshana.app.data.BirthProfile
import com.kshana.app.data.EngineSettings
import com.kshana.app.data.Place
import com.kshana.app.engine.EngineProvider
import com.kshana.app.engine.Instants
import com.kshana.app.ui.theme.KshanaColors
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import uniffi.kshana_ffi.ExactInstant
import uniffi.kshana_ffi.HorizonRecord
import uniffi.kshana_ffi.MomentRecord
import java.time.Duration
import java.time.Instant
import java.time.format.DateTimeFormatter

/** Shared pieces of the plain-language screens. Nothing here judges a chart. */

internal val TIME_HM: DateTimeFormatter = DateTimeFormatter.ofPattern("HH:mm")
internal val DAY_MON: DateTimeFormatter = DateTimeFormatter.ofPattern("d MMM")
internal val WEEKDAY_DAY: DateTimeFormatter = DateTimeFormatter.ofPattern("EEE d MMM")

internal val LIFE_ORDER = listOf("Love", "Career", "Money", "Health", "Family", "Study", "Home", "Travel", "Children")

internal fun bandColor(band: String): Color = when (band) {
    "bright" -> KshanaColors.Gold
    "good" -> KshanaColors.Leaf
    "steady" -> KshanaColors.Sky
    "slow" -> KshanaColors.Muted
    else -> KshanaColors.Ember
}

internal fun bandWord(band: String): String = when (band) {
    "bright" -> "Bright"
    "good" -> "Good"
    "steady" -> "Steady"
    "slow" -> "Slow"
    else -> "Hold"
}

internal fun scoreColor(score: Double): Color = when {
    score >= 70 -> KshanaColors.Gold
    score >= 56 -> KshanaColors.Leaf
    score >= 44 -> KshanaColors.Sky
    score >= 32 -> KshanaColors.Muted
    else -> KshanaColors.Ember
}

internal fun lifeGlyph(life: String): String = when (life) {
    "Love" -> "♥"
    "Career" -> "▲"
    "Money" -> "◆"
    "Health" -> "✚"
    "Family" -> "☗"
    "Study" -> "✎"
    "Home" -> "⌂"
    "Travel" -> "➶"
    "Children" -> "✿"
    else -> "•"
}

internal fun lifeQuestion(life: String): String = when (life) {
    "Love" -> "How is love for me now?"
    "Career" -> "How is work going to go?"
    "Money" -> "How is money for me?"
    "Health" -> "How is my health?"
    "Family" -> "How is home and family?"
    "Study" -> "How will study go?"
    "Home" -> "What about house and property?"
    "Travel" -> "Is it a time to travel?"
    "Children" -> "What about the children?"
    else -> life
}

/** An exact instant as a java.time instant on the device clock. */
internal fun ExactInstant.toJava(): Instant = Instants.toLocal(this).toInstant()

/** "12:34" under an hour, "1h 05m" under a day, else "2d 3h". */
internal fun countdown(from: Instant, to: Instant): String {
    val s = Duration.between(from, to).seconds.coerceAtLeast(0)
    return when {
        s < 3600 -> "%02d:%02d".format(s / 60, s % 60)
        s < 86_400 -> "%dh %02dm".format(s / 3600, (s % 3600) / 60)
        else -> "%dd %dh".format(s / 86_400, (s % 86_400) / 3600)
    }
}

internal fun hm(e: ExactInstant): String = localText(e, TIME_HM)
internal fun dayMon(e: ExactInstant): String = localText(e, DAY_MON)

internal val WEEKDAY_DAY_YEAR: DateTimeFormatter = DateTimeFormatter.ofPattern("EEE d MMM uuuu")

/** "Sat 21 Nov", or "Mon 16 Oct 2028" when the day is not in the current year. */
internal fun whenText(e: ExactInstant): String {
    val local = Instants.toLocal(e)
    return if (local.year == java.time.ZonedDateTime.now().year) local.format(WEEKDAY_DAY) else local.format(WEEKDAY_DAY_YEAR)
}

internal suspend fun loadMoment(
    context: Context,
    profile: BirthProfile,
    place: Place,
    settings: EngineSettings,
    at: ExactInstant = Instants.now(),
): MomentRecord {
    val engine = EngineProvider.get(context)
    return withContext(Dispatchers.Default) {
        engine.forecastMoment(
            profile.birthInstant(), profile.latitude, profile.longitude,
            place.latitude, place.longitude,
            settings.houseSystem, settings.ayanamsa, at,
        )
    }
}

internal suspend fun loadHorizon(
    context: Context,
    profile: BirthProfile,
    place: Place,
    settings: EngineSettings,
    from: ExactInstant,
    stepDays: Double,
    count: Int,
): HorizonRecord {
    val engine = EngineProvider.get(context)
    return withContext(Dispatchers.Default) {
        engine.forecastHorizon(
            profile.birthInstant(), profile.latitude, profile.longitude,
            place.latitude, place.longitude,
            settings.houseSystem, settings.ayanamsa, from, stepDays, count.toUInt(),
        )
    }
}
