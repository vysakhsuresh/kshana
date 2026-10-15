package com.kshana.app.ui.theme

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Typography
import androidx.compose.material3.darkColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.sp

/** The design canvas palette: night ground, one gold accent, a sky blue for data. */
object KshanaColors {
    val Night = Color(0xFF080A14)
    val Surface = Color(0xFF0E1120)
    val Line = Color(0xFF2A2E44)
    val LineSoft = Color(0xFF1B1F30)
    val Text = Color(0xFFEFE9DC)
    val TextSoft = Color(0xFFC9C3B4)
    val Muted = Color(0xFF9A9EB0)
    val Gold = Color(0xFFE3B96A)
    val Sky = Color(0xFFA9C4E6)
}

private val scheme = darkColorScheme(
    primary = KshanaColors.Gold,
    onPrimary = KshanaColors.Night,
    background = KshanaColors.Night,
    onBackground = KshanaColors.Text,
    surface = KshanaColors.Surface,
    onSurface = KshanaColors.Text,
    outline = KshanaColors.Line,
)

/** Display serif, UI sans, mono for every number. System families until the bundled
 *  fonts (Cormorant Garamond, Manrope, IBM Plex Mono) are added under res/font. */
val KshanaType = Typography(
    displayLarge = TextStyle(fontFamily = FontFamily.Serif, fontWeight = FontWeight.Medium, fontSize = 40.sp, lineHeight = 42.sp),
    displayMedium = TextStyle(fontFamily = FontFamily.Serif, fontWeight = FontWeight.Medium, fontSize = 30.sp, lineHeight = 32.sp),
    titleLarge = TextStyle(fontFamily = FontFamily.Serif, fontWeight = FontWeight.Medium, fontSize = 20.sp),
    bodyMedium = TextStyle(fontFamily = FontFamily.SansSerif, fontSize = 13.sp, lineHeight = 20.sp),
    labelSmall = TextStyle(fontFamily = FontFamily.SansSerif, fontSize = 10.sp, letterSpacing = 1.4.sp),
    bodySmall = TextStyle(fontFamily = FontFamily.Monospace, fontSize = 11.sp),
    headlineLarge = TextStyle(fontFamily = FontFamily.Monospace, fontWeight = FontWeight.Medium, fontSize = 44.sp),
)

@Composable
fun KshanaTheme(content: @Composable () -> Unit) {
    // Always night: the sky is the subject.
    isSystemInDarkTheme()
    MaterialTheme(colorScheme = scheme, typography = KshanaType, content = content)
}
