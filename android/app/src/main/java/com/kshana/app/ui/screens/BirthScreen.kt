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
import com.kshana.app.data.Place
import com.kshana.app.data.ProfileStore
import com.kshana.app.data.Uncertainty
import com.kshana.app.ui.theme.KshanaColors
import java.time.DateTimeException
import java.time.LocalDateTime
import java.time.ZoneId

private data class LevelCopy(val headline: String, val detail: String)

/** Ascendant ≈ 0°15′ per minute of clock time; the odds below follow that rate. */
private val LEVEL_COPY = listOf(
    LevelCopy("The sky is fixed.", "The Ascendant moves about 0°15′ per minute. With an exact time every cusp is a point, not a band."),
    LevelCopy("A quarter degree of doubt.", "The Ascendant drifts about 0°15′ across this window. Roughly one in a hundred and twenty people change rising sign; houses and the Moon hold."),
    LevelCopy("A degree and a quarter.", "The Ascendant drifts about 1°15′. Roughly one in twenty-four people change rising sign; the Moon stays within 0°03′."),
    LevelCopy("Nearly four degrees.", "The Ascendant drifts about 3°45′. One in eight people change rising sign here, and house cusps shift visibly."),
    LevelCopy("Half a sign of doubt.", "The Ascendant drifts about 15°. One in two people change rising sign; houses are read with caution, planets by sign are safe."),
    LevelCopy("More than a sign.", "The Ascendant drifts about 45°, more than a whole sign. The rising sign cannot be known; the Moon still holds to within 1°40′."),
    LevelCopy("No time at all.", "Without a time, planets by sign and the Moon's nakshatra mostly survive. The Ascendant, the houses and the dasha start date do not."),
)

/**
 * "The moment you arrived": date, time, place, stated uncertainty. Saves the profile and
 * hands control back so the caller can navigate to the chart.
 */
@Composable
fun BirthScreen(existing: BirthProfile?, onCast: (BirthProfile) -> Unit) {
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
            name = name.trim().ifEmpty { "You" },
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
        Text("BIRTH", style = MaterialTheme.typography.labelSmall, color = KshanaColors.Gold)
        Text("The moment you arrived", style = MaterialTheme.typography.displayMedium, color = KshanaColors.Text)
        Text(
            "Date, clock time and place, as written on the record. Say how sure the time is; the sky is drawn to that width.",
            style = MaterialTheme.typography.bodyMedium, color = KshanaColors.Muted,
        )

        Field(name, { name = it }, "Name", Modifier.fillMaxWidth())

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
                        Text(p.name, fontFamily = FontFamily.Serif, fontSize = 17.sp, color = KshanaColors.Text, modifier = Modifier.weight(1f))
                        Text("%.2f° %.2f°".format(p.latitude, p.longitude), style = MaterialTheme.typography.bodySmall, color = KshanaColors.Sky)
                    }
                }
                HorizontalDivider(color = KshanaColors.LineSoft)
            }
        }
        Text(placeName, fontFamily = FontFamily.Serif, fontSize = 20.sp, color = KshanaColors.Text)
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
                    fontFamily = FontFamily.Monospace,
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
        GoldButton("CAST THE SKY", onClick = {
            val p = build()
            if (p != null) {
                ProfileStore.save(context, p)
                onCast(p)
            }
        }, modifier = Modifier.fillMaxWidth())
        Text("Everything is computed on this phone. Nothing leaves it.", style = MaterialTheme.typography.bodySmall, color = KshanaColors.Muted, modifier = Modifier.padding(top = 4.dp))
    }
}

@Composable
private fun Field(value: String, onChange: (String) -> Unit, label: String, modifier: Modifier = Modifier) {
    OutlinedTextField(
        value = value,
        onValueChange = onChange,
        label = { Text(label, style = MaterialTheme.typography.labelSmall) },
        singleLine = true,
        textStyle = TextStyle(fontFamily = FontFamily.SansSerif, fontSize = 15.sp, color = KshanaColors.Text),
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
        textStyle = TextStyle(fontFamily = FontFamily.Monospace, fontSize = 15.sp, color = KshanaColors.Text),
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
