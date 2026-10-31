/*
    LibrePods - AirPods liberated from Apple’s ecosystem
    Copyright (C) 2025 LibrePods contributors

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

package me.kavishdevar.librepods.presentation.theme

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.Typography
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.Font
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontVariation
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.googlefonts.GoogleFont
import androidx.compose.ui.tooling.preview.Devices.PIXEL_9_PRO_XL
import androidx.compose.ui.tooling.preview.Preview
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import me.kavishdevar.librepods.R

val sfProFamily = FontFamily(Font(R.font.sf_pro))

val AppleTypography = Typography().run {
    copy(
        displayLarge = displayLarge.copy(fontFamily = sfProFamily),
        displayMedium = displayMedium.copy(fontFamily = sfProFamily),
        displaySmall = displaySmall.copy(fontFamily = sfProFamily),

        headlineLarge = headlineLarge.copy(fontFamily = sfProFamily),
        headlineMedium = headlineMedium.copy(fontFamily = sfProFamily),
        headlineSmall = headlineSmall.copy(fontFamily = sfProFamily),

        titleLarge = titleLarge.copy(fontFamily = sfProFamily),
        titleMedium = titleMedium.copy(fontFamily = sfProFamily),
        titleSmall = titleSmall.copy(fontFamily = sfProFamily),

        bodyLarge = bodyLarge.copy(fontFamily = sfProFamily),
        bodyMedium = bodyMedium.copy(
            fontFamily = sfProFamily,
            fontSize = 16.sp
        ),
        bodySmall = bodySmall.copy(
            fontFamily = sfProFamily,
            fontSize = 14.sp,
            lineHeight = 18.sp
        ),

        labelLarge = labelLarge.copy(fontFamily = sfProFamily),

        labelMedium = labelMedium.copy(
            fontFamily = sfProFamily,
            fontSize = 16.sp,
        ),
        labelMediumEmphasized = labelMediumEmphasized.copy(
            fontFamily = sfProFamily,
            fontSize = 16.sp,
            fontWeight = FontWeight.Bold
        ),
        labelSmallEmphasized = labelSmallEmphasized.copy(
            fontFamily = sfProFamily,
            fontSize = 14.sp,
            fontWeight = FontWeight.Bold
        )
    )
}

val provider = GoogleFont.Provider(
    providerAuthority = "com.google.android.gms.fonts",
    providerPackage = "com.google.android.gms",
    certificates = R.array.com_google_android_gms_fonts_certs
)

private fun robotoFlex(
    wght: Float = 400f,
    slnt: Float = 0f,
    grad: Float = 0f,
    wdth: Float = 100f,
    xtra: Float = 468f,
    xopq: Float = 96f,
    yopq: Float = 79f,
) = FontFamily(
    androidx.compose.ui.text.googlefonts.Font(
//    Font(
//        resId = R.font.roboto_flex,
        googleFont = GoogleFont("Roboto Flex"),
        fontProvider = provider,
        variationSettings = FontVariation.Settings(
            FontVariation.Setting("wght", wght),
            FontVariation.Setting("wdth", wdth),
            FontVariation.Setting("slnt", slnt),
            FontVariation.Setting("grad", grad),
            FontVariation.Setting("xtra", xtra),
            FontVariation.Setting("xopq", xopq),
            FontVariation.Setting("yopq", yopq),
        )
    )
)

val display = robotoFlex(
    wght = 800f,
    grad = 100f,
    wdth = 100f
)

val displayEmphasized = robotoFlex(
    wght = 1000f,
    slnt = -2f,
    grad = 150f,
    wdth = 150f,
)

val body = robotoFlex()

val bodyEmphasized = robotoFlex(
    wght = 600f,
    wdth = 130f,
    grad = 75f,
)

val label = robotoFlex(
    wght = 450f,
    grad = 50f
)

val labelEmphasized = robotoFlex(
    wght = 600f,
    wdth = 140f,
    grad = 75f
)


val MaterialTypography = Typography().run {
    // Standard Material 3 type scale (sizes untouched), set in Roboto Flex.
    val heading = robotoFlex(wght = 500f)
    val headingEmphasized = robotoFlex(wght = 650f, wdth = 110f)
    val text = robotoFlex(wght = 400f)
    val textEmphasized = robotoFlex(wght = 550f)
    val labels = robotoFlex(wght = 500f)
    val labelsEmphasized = robotoFlex(wght = 650f)
    copy(
        displayLarge = displayLarge.copy(fontFamily = heading),
        displayMedium = displayMedium.copy(fontFamily = heading),
        displaySmall = displaySmall.copy(fontFamily = heading),
        headlineLarge = headlineLarge.copy(fontFamily = heading),
        headlineMedium = headlineMedium.copy(fontFamily = heading),
        headlineSmall = headlineSmall.copy(fontFamily = heading),
        titleLarge = titleLarge.copy(fontFamily = heading),
        titleMedium = titleMedium.copy(fontFamily = heading),
        titleSmall = titleSmall.copy(fontFamily = heading),
        bodyLarge = bodyLarge.copy(fontFamily = text),
        bodyMedium = bodyMedium.copy(fontFamily = text),
        bodySmall = bodySmall.copy(fontFamily = text),
        labelLarge = labelLarge.copy(fontFamily = labels),
        labelMedium = labelMedium.copy(fontFamily = labels),
        labelSmall = labelSmall.copy(fontFamily = labels),
        displayLargeEmphasized = displayLargeEmphasized.copy(fontFamily = headingEmphasized),
        displayMediumEmphasized = displayMediumEmphasized.copy(fontFamily = headingEmphasized),
        displaySmallEmphasized = displaySmallEmphasized.copy(fontFamily = headingEmphasized),
        headlineLargeEmphasized = headlineLargeEmphasized.copy(fontFamily = headingEmphasized),
        headlineMediumEmphasized = headlineMediumEmphasized.copy(fontFamily = headingEmphasized),
        headlineSmallEmphasized = headlineSmallEmphasized.copy(fontFamily = headingEmphasized),
        titleLargeEmphasized = titleLargeEmphasized.copy(fontFamily = headingEmphasized),
        titleMediumEmphasized = titleMediumEmphasized.copy(fontFamily = headingEmphasized),
        titleSmallEmphasized = titleSmallEmphasized.copy(fontFamily = headingEmphasized),
        bodyLargeEmphasized = bodyLargeEmphasized.copy(fontFamily = textEmphasized),
        bodyMediumEmphasized = bodyMediumEmphasized.copy(fontFamily = textEmphasized),
        bodySmallEmphasized = bodySmallEmphasized.copy(fontFamily = textEmphasized),
        labelLargeEmphasized = labelLargeEmphasized.copy(fontFamily = labelsEmphasized),
        labelMediumEmphasized = labelMediumEmphasized.copy(fontFamily = labelsEmphasized),
        labelSmallEmphasized = labelSmallEmphasized.copy(fontFamily = labelsEmphasized),
    )
}

@Preview(
    name = "Typography Showcase",
    showBackground = true,
    device = PIXEL_9_PRO_XL
)
@Composable
private fun TypographyPreview() {
    LibrePodsTheme (m3eEnabled = true) {
        Box(
            modifier = Modifier
                .fillMaxSize()
                .background(MaterialTheme.colorScheme.surfaceContainer, RoundedCornerShape(28.dp))
                .padding(24.dp)
        ) {
            Column(
                verticalArrangement = Arrangement.spacedBy(12.dp)
            ) {
                Text(
                    "Display Large",
                    style = MaterialTheme.typography.displayLarge
                )

                Text(
                    "Display Large Emphasized",
                    style = MaterialTheme.typography.displayLargeEmphasized
                )

                Text(
                    "Display Medium",
                    style = MaterialTheme.typography.displayMedium
                )

                Text(
                    "Display Medium Emphasized",
                    style = MaterialTheme.typography.displayMediumEmphasized
                )

                Text(
                    "Display Small",
                    style = MaterialTheme.typography.displaySmall
                )

                Text(
                    "Display Small Emphasized",
                    style = MaterialTheme.typography.displaySmallEmphasized
                )

                HorizontalDivider()

                Text(
                    "Body Large",
                    style = MaterialTheme.typography.bodyLarge
                )

                Text(
                    "Body Large Emphasized",
                    style = MaterialTheme.typography.bodyLargeEmphasized
                )

                Text(
                    "Body Medium",
                    style = MaterialTheme.typography.bodyMedium
                )

                Text(
                    "Body Medium Emphasized",
                    style = MaterialTheme.typography.bodyMediumEmphasized
                )

                Text(
                    "Body Small",
                    style = MaterialTheme.typography.bodySmall
                )

                Text(
                    "Body Small Emphasized",
                    style = MaterialTheme.typography.bodySmallEmphasized
                )

                HorizontalDivider()

                Text(
                    "Label Large",
                    style = MaterialTheme.typography.labelLarge
                )

                Text(
                    "Label Large Emphasized",
                    style = MaterialTheme.typography.labelLargeEmphasized
                )

                Text(
                    "Label Medium",
                    style = MaterialTheme.typography.labelMedium
                )

                Text(
                    "Label Medium Emphasized",
                    style = MaterialTheme.typography.labelMediumEmphasized
                )

                Text(
                    "Label Small",
                    style = MaterialTheme.typography.labelSmall
                )

                Text(
                    "Label Small Emphasized",
                    style = MaterialTheme.typography.labelSmallEmphasized
                )
            }
        }
    }
}
