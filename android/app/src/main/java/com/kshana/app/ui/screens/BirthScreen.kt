package com.kshana.app.ui.screens

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.OutlinedTextFieldDefaults
import androidx.compose.material3.Slider
import androidx.compose.material3.SliderDefaults
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.kshana.app.data.BirthProfile
import com.kshana.app.data.FamilyStore
import com.kshana.app.data.Place
import com.kshana.app.data.ProfileStore
import com.kshana.app.data.Uncertainty
import com.kshana.app.net.searchPlaces
import com.kshana.app.ui.theme.KshanaColors
import kotlinx.coroutines.launch
import java.time.DateTimeException
import java.time.LocalDateTime
import java.time.ZoneId
import com.kshana.app.ui.sky.KshanaFonts

private data class LevelCopy(val headline: String, val detail: String)

/** Ascendant ≈ 0°15′ per minute of clock time; the odds below follow that rate. */
private val LEVEL_COPY = listOf(
    LevelCopy("The time is exact.", "Every reading is as sharp as it can be, down to the hour."),
    LevelCopy("A little doubt.", "Everything holds; a few hour-by-hour readings may shift slightly."),
    LevelCopy("Some doubt.", "Daily and monthly readings hold; the exact hour of a few turns may move."),
    LevelCopy("Real doubt.", "Most readings hold; some dates may move by a few days."),
    LevelCopy("Large doubt.", "The broad picture holds; dates for some parts of life may be off by weeks."),
    LevelCopy("Very large doubt.", "Only the broad picture holds. Use Find your exact birth time under Explore to sharpen it."),
    LevelCopy("No time at all.", "The broad seasons and your daily luck hold. Dates for each part of life cannot be trusted."),
)

/**
 * "The moment you arrived": date, time, place, stated uncertainty. Saves the profile and
 * hands control back so the caller can navigate to the chart.
 */
@Composable
fun BirthScreen(
    existing: BirthProfile?,
    onCast: (BirthProfile) -> Unit,
    heading: String = "BIRTH",
    title: String = "The moment you arrived",
    intro: String = "Date, clock time and place, as written on the record. Say how sure the time is; the readings are drawn to that width.",
    nameLabel: String = "Name",
    button: String = "SEE MY READING",
    persist: Boolean = true,
    onBack: (() -> Unit)? = null,
) {
    val context = LocalContext.current
    val initialPlace = existing?.place() ?: ProfileStore.kochi

    var name by remember { mutableStateOf(existing?.name ?: "") }
    var year by remember { mutableStateOf(existing?.year?.toString() ?: "") }
    var month by remember { mutableStateOf(existing?.month?.toString() ?: "") }
    var day by remember { mutableStateOf(existing?.day?.toString() ?: "") }
    var hour by remember { mutableStateOf(existing?.hour?.toString() ?: "") }
    var minute by remember { mutableStateOf(existing?.minute?.toString() ?: "") }
    var second by remember { mutableStateOf(existing?.second?.toString() ?: "0") }
    var level by remember { mutableIntStateOf(existing?.uncertaintyLevel ?: 0) }
    var query by remember { mutableStateOf("") }
    var placeName by remember { mutableStateOf(initialPlace.name) }
    var latitude by remember { mutableStateOf("%.4f".format(initialPlace.latitude)) }
    var longitude by remember { mutableStateOf("%.4f".format(initialPlace.longitude)) }
    var zoneId by remember { mutableStateOf(initialPlace.zoneId) }
    var problem by remember { mutableStateOf<String?>(null) }

    // Online place search (any place on Earth). Off until the user allows online features.
    val scope = rememberCoroutineScope()
    var onlineOn by remember { mutableStateOf(FamilyStore.onlineAllowed(context)) }
    var earthQuery by remember { mutableStateOf("") }
    var earthResults by remember { mutableStateOf<List<Place>>(emptyList()) }
    var earthError by remember { mutableStateOf<String?>(null) }
    var searching by remember { mutableStateOf(false) }

    val matches = remember(query) {
        if (query.isBlank()) emptyList()
        else ProfileStore.places.filter { it.name.contains(query.trim(), ignoreCase = true) }.take(6)
    }

    fun choose(p: Place) {
        placeName = p.name
        latitude = "%.4f".format(p.latitude)
        longitude = "%.4f".format(p.longitude)
        zoneId = p.zoneId
        query = ""
        earthResults = emptyList()
        earthError = null
    }

    fun searchEarth() {
        val q = earthQuery.trim()
        if (q.isEmpty() || searching) return
        scope.launch {
            searching = true
            try {
                val found = searchPlaces(context, q)
                earthResults = found
                earthError = if (found.isEmpty()) "No place found for \"$q\"." else null
            } catch (e: Exception) {
                earthResults = emptyList()
                earthError = e.message ?: e.toString()
            }
            searching = false
        }
    }

    fun build(): BirthProfile? {
        val y = year.toIntOrNull(); val mo = month.toIntOrNull(); val d = day.toIntOrNull()
        val h = hour.toIntOrNull(); val mi = minute.toIntOrNull(); val s = second.toIntOrNull() ?: 0
        val lat = latitude.toDoubleOrNull(); val lon = longitude.toDoubleOrNull()
        if (y == null || mo == null || d == null) { problem = "The date needs a year, a month and a day."; return null }
        if (h == null || mi == null) { problem = "The time needs an hour and a minute. If it is unknown, enter 12:00 and set the slider to unknown."; return null }
        if (lat == null || lon == null || lat < -90 || lat > 90 || lon < -180 || lon > 180) { problem = "Latitude must be −90…90 and longitude −180…180."; return null }
        try { LocalDateTime.of(y, mo, d, h, mi, s) } catch (e: DateTimeException) { problem = "That date and time do not exist on the calendar."; return null }
        try { ZoneId.of(zoneId.trim()) } catch (e: DateTimeException) { problem = "Unknown time zone id. Use an IANA name such as Asia/Kolkata."; return null }
        problem = null
        return BirthProfile(
            name = name.trim().ifEmpty { if (persist) "You" else "Partner" },
            year = y, month = mo, day = d, hour = h, minute = mi, second = s,
            uncertaintyLevel = level,
            latitude = lat, longitude = lon,
            placeName = placeName.trim().ifEmpty { "%.2f, %.2f".format(lat, lon) },
            zoneId = zoneId.trim(),
        )
    }

    Column(
        Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(horizontal = 24.dp).padding(top = 48.dp, bottom = 40.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
            Text(heading, style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
            if (onBack != null) Text("BACK", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Muted, modifier = Modifier.clickable(onClick = onBack).padding(12.dp))
        }
        Text(title, style = MaterialTheme.typography.displayMedium, color = KshanaColors.Text)
        Text(intro, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)

        Field(name, { name = it }, nameLabel, Modifier.fillMaxWidth())

        SectionLabel("Date")
        Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
            NumberField(year, { year = it.filter(Char::isDigit).take(4) }, "Year", Modifier.weight(1.4f))
            NumberField(month, { month = it.filter(Char::isDigit).take(2) }, "Month", Modifier.weight(1f))
            NumberField(day, { day = it.filter(Char::isDigit).take(2) }, "Day", Modifier.weight(1f))
        }

        SectionLabel("Clock time")
        Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
            NumberField(hour, { hour = it.filter(Char::isDigit).take(2) }, "Hour (0–23)", Modifier.weight(1f))
            NumberField(minute, { minute = it.filter(Char::isDigit).take(2) }, "Minute", Modifier.weight(1f))
            NumberField(second, { second = it.filter(Char::isDigit).take(2) }, "Second", Modifier.weight(1f))
        }

        SectionLabel("Place")
        Field(query, { query = it }, "Search a city", Modifier.fillMaxWidth())
        if (matches.isNotEmpty()) {
            Column(Modifier.fillMaxWidth()) {
                for (p in matches) {
                    HorizontalDivider(color = KshanaColors.LineSoft)
                    Row(
                        Modifier.fillMaxWidth().heightIn(min = 44.dp).clickable { choose(p) }.padding(vertical = 8.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Text(p.name, fontFamily = KshanaFonts.serif, fontSize = 17.sp, color = KshanaColors.Text, modifier = Modifier.weight(1f))
                        Text("%.2f° %.2f°".format(p.latitude, p.longitude), style = MaterialTheme.typography.bodySmall, color = KshanaColors.Sky)
                    }
                }
                HorizontalDivider(color = KshanaColors.LineSoft)
            }
        }

        SectionLabel("Any place on Earth")
        if (!onlineOn) {
            Text("Online place search is off", style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted)
            QuietButton("ALLOW ONLINE FEATURES", onClick = {
                FamilyStore.saveOnlineAllowed(context, true)
                onlineOn = true
            })
        } else {
            Row(horizontalArrangement = Arrangement.spacedBy(10.dp), verticalAlignment = Alignment.CenterVertically) {
                Field(earthQuery, { earthQuery = it }, "Town, city or village", Modifier.weight(1f))
                QuietButton(if (searching) "SEARCHING…" else "SEARCH", onClick = { searchEarth() })
            }
            Text("Looked up on OpenStreetMap; the time zone is derived from the country and longitude. Check it below.", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted)
            ErrorLine(earthError)
            if (earthResults.isNotEmpty()) {
                Column(Modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                    for (p in earthResults) {
                        Chip("${p.name}  ·  ${"%.2f".format(p.latitude)}° ${"%.2f".format(p.longitude)}°", selected = false, onClick = { choose(p) })
                    }
                }
            }
        }

        Text(placeName, fontFamily = KshanaFonts.serif, fontSize = 20.sp, color = KshanaColors.Text)
        Row(horizontalArrangement = Arrangement.spacedBy(10.dp)) {
            NumberField(latitude, { latitude = it.filter { c -> c.isDigit() || c == '.' || c == '-' } }, "Latitude °N", Modifier.weight(1f), decimal = true)
            NumberField(longitude, { longitude = it.filter { c -> c.isDigit() || c == '.' || c == '-' } }, "Longitude °E", Modifier.weight(1f), decimal = true)
        }
        Field(zoneId, { zoneId = it }, "Time zone (IANA id)", Modifier.fillMaxWidth())
        Field(placeName, { placeName = it }, "Place name", Modifier.fillMaxWidth())

        SectionLabel("How sure is the time?")
        Slider(
            value = level.toFloat(),
            onValueChange = { level = it.toInt().coerceIn(0, 6) },
            valueRange = 0f..6f,
            steps = 5,
            colors = SliderDefaults.colors(
                thumbColor = KshanaColors.Gold,
                activeTrackColor = KshanaColors.Gold,
                inactiveTrackColor = KshanaColors.Line,
                activeTickColor = KshanaColors.Night,
                inactiveTickColor = KshanaColors.Muted,
            ),
            modifier = Modifier.fillMaxWidth().height(44.dp),
        )
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
            for ((i, l) in Uncertainty.labels.withIndex()) {
                Text(
                    l,
                    fontFamily = KshanaFonts.mono,
                    fontSize = 9.sp,
                    color = if (i == level) KshanaColors.Gold else KshanaColors.Muted,
                    modifier = Modifier.clickable { level = i }.padding(vertical = 4.dp),
                )
            }
        }
        Text(LEVEL_COPY[level].headline, style = MaterialTheme.typography.titleLarge, color = KshanaColors.Text)
        Text(LEVEL_COPY[level].detail, style = MaterialTheme.typography.bodyMedium, color = KshanaColors.TextSoft)

        Spacer(Modifier.height(8.dp))
        problem?.let { Text(it, color = KshanaColors.Sky, style = MaterialTheme.typography.bodyMedium) }
        GoldButton(button, onClick = {
            val p = build()
            if (p != null) {
                if (persist) ProfileStore.save(context, p)
                onCast(p)
            }
        }, modifier = Modifier.fillMaxWidth())
        Text("Everything is worked out on this phone. Nothing leaves it.", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted, modifier = Modifier.padding(top = 4.dp))
    }
}

@Composable
private fun Field(value: String, onChange: (String) -> Unit, label: String, modifier: Modifier = Modifier) {
    OutlinedTextField(
        value = value,
        onValueChange = onChange,
        label = { Text(label, style = MaterialTheme.typography.labelSmall) },
        singleLine = true,
        textStyle = TextStyle(fontFamily = KshanaFonts.sans, fontSize = 15.sp, color = KshanaColors.Text),
        colors = fieldColors(),
        modifier = modifier,
    )
}

@Composable
private fun NumberField(value: String, onChange: (String) -> Unit, label: String, modifier: Modifier = Modifier, decimal: Boolean = false) {
    OutlinedTextField(
        value = value,
        onValueChange = onChange,
        label = { Text(label, style = MaterialTheme.typography.labelSmall) },
        singleLine = true,
        keyboardOptions = KeyboardOptions(keyboardType = if (decimal) KeyboardType.Decimal else KeyboardType.Number),
        textStyle = TextStyle(fontFamily = KshanaFonts.mono, fontSize = 15.sp, color = KshanaColors.Text),
        colors = fieldColors(),
        modifier = modifier,
    )
}

@Composable
private fun fieldColors() = OutlinedTextFieldDefaults.colors(
    focusedTextColor = KshanaColors.Text,
    unfocusedTextColor = KshanaColors.Text,
    cursorColor = KshanaColors.Gold,
    focusedBorderColor = KshanaColors.Gold,
    unfocusedBorderColor = KshanaColors.Line,
    focusedLabelColor = KshanaColors.Gold,
    unfocusedLabelColor = KshanaColors.Muted,
    focusedContainerColor = KshanaColors.Surface,
    unfocusedContainerColor = KshanaColors.Surface,
)
