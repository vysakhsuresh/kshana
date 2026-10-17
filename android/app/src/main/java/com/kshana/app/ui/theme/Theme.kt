package com.kshana.app.ui.theme

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Typography
import androidx.compose.material3.darkColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.Font
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.sp
import com.kshana.app.R

/** The design canvas palette: night ground, one gold accent, a sky blue for data. */
object KshanaColors {
    val Night = Color(0xFF07091A)
    val Surface = Color(0xFF0E1224)
    val Line = Color(0xFF342F48)
    val LineSoft = Color(0xFF221F36)
    val Text = Color(0xFFEFE9DC)
    val TextSoft = Color(0xFFC9C3B4)
    val Muted = Color(0xFF9A9EB0)
    val Gold = Color(0xFFE3B96A)
    val Sky = Color(0xFFA9C4E6)
    val Ember = Color(0xFFE07A5F)
    val Leaf = Color(0xFF8FBF9F)
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

/** The UI sans: Space Grotesk, bundled under res/font (the same family LayerLink uses). */
val Grotesk = FontFamily(
    Font(R.font.space_grotesk_regular, FontWeight.Normal),
    Font(R.font.space_grotesk_medium, FontWeight.Medium),
    Font(R.font.space_grotesk_semibold, FontWeight.SemiBold),
    Font(R.font.space_grotesk_bold, FontWeight.Bold),
)

/** Display serif for the sentences, Space Grotesk for every label, chip and option,
 *  mono for every number. */
val KshanaType = Typography(
    displayLarge = TextStyle(fontFamily = FontFamily.Serif, fontWeight = FontWeight.Medium, fontSize = 40.sp, lineHeight = 42.sp),
    displayMedium = TextStyle(fontFamily = FontFamily.Serif, fontWeight = FontWeight.Medium, fontSize = 30.sp, lineHeight = 32.sp),
    titleLarge = TextStyle(fontFamily = FontFamily.Serif, fontWeight = FontWeight.Medium, fontSize = 20.sp),
    titleMedium = TextStyle(fontFamily = Grotesk, fontWeight = FontWeight.Medium, fontSize = 15.sp, lineHeight = 20.sp),
    bodyMedium = TextStyle(fontFamily = Grotesk, fontSize = 13.sp, lineHeight = 20.sp),
    bodyLarge = TextStyle(fontFamily = Grotesk, fontSize = 15.sp, lineHeight = 22.sp),
    labelSmall = TextStyle(fontFamily = Grotesk, fontWeight = FontWeight.Medium, fontSize = 10.sp, letterSpacing = 1.4.sp),
    labelMedium = TextStyle(fontFamily = Grotesk, fontWeight = FontWeight.Medium, fontSize = 12.sp, letterSpacing = 1.sp),
    bodySmall = TextStyle(fontFamily = FontFamily.Monospace, fontSize = 11.sp),
    headlineLarge = TextStyle(fontFamily = FontFamily.Monospace, fontWeight = FontWeight.Medium, fontSize = 44.sp),
)

@Composable
fun KshanaTheme(content: @Composable () -> Unit) {
    // Always night: the sky is the subject.
    isSystemInDarkTheme()
    MaterialTheme(colorScheme = scheme, typography = KshanaType, content = content)
}
