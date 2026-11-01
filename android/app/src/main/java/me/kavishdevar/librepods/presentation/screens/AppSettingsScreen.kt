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


package me.kavishdevar.librepods.presentation.screens

import android.content.Intent
import android.os.Build
import android.widget.Toast
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Slider
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.core.content.edit
import androidx.core.net.toUri
import androidx.lifecycle.viewmodel.compose.viewModel
import me.kavishdevar.librepods.BuildConfig
import me.kavishdevar.librepods.R
import me.kavishdevar.librepods.presentation.components.LeadTone
import me.kavishdevar.librepods.presentation.components.RadioRow
import me.kavishdevar.librepods.presentation.components.SectionHeader
import me.kavishdevar.librepods.presentation.components.SettingsGroup
import me.kavishdevar.librepods.presentation.components.SettingsRow
import me.kavishdevar.librepods.presentation.components.SwitchRow
import me.kavishdevar.librepods.presentation.viewmodel.AppSettingsViewModel
import me.kavishdevar.librepods.services.ServiceManager
import me.kavishdevar.librepods.utils.XposedState

private const val UPSTREAM_URL = "https://github.com/librepods-org/librepods"

@Composable
fun AppSettingsScreen(
    viewModel: AppSettingsViewModel = viewModel(),
    navigateToTroubleshooting: () -> Unit,
    navigateToOpenSourceLicenses: () -> Unit,
    navigateToReleaseNotesScreen: () -> Unit
) {
    val context = LocalContext.current
    val state by viewModel.uiState.collectAsState()
    val prefs = remember { context.getSharedPreferences("settings", android.content.Context.MODE_PRIVATE) }
    var connectionNotifications by remember { mutableStateOf(prefs.getBoolean("connection_notifications", true)) }
    var shadeCard by remember { mutableStateOf(prefs.getBoolean("shade_card", true)) }

    // "Automatically" mirrors iOS: take the AirPods over when this phone starts playing or rings.
    // "When last connected" only reconnects if this phone was the last one used.
    val automatic = state.takeoverWhenMediaStart || state.takeoverWhenIdle
    fun setAutomatic(on: Boolean) {
        viewModel.setTakeoverWhenDisconnected(true)
        viewModel.setTakeoverWhenIdle(on)
        viewModel.setTakeoverWhenRingingCall(on)
        viewModel.setTakeoverWhenMediaStart(on)
        viewModel.setTakeoverWhenMusic(false)
    }
    fun open(url: String) = context.startActivity(Intent(Intent.ACTION_VIEW, url.toUri()).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))

    Column(
        Modifier
            .fillMaxSize()
            .background(MaterialTheme.colorScheme.surfaceContainer)
            .verticalScroll(rememberScrollState())
            .padding(horizontal = 16.dp)
    ) {
        Spacer(Modifier.height(8.dp))

        SectionHeader(stringResource(R.string.ui_connect_to_this_phone), Modifier.padding(top = 0.dp))
        SettingsGroup(
            rows = arrayOf(
                { m -> RadioRow(m, stringResource(R.string.ui_connect_automatically), stringResource(R.string.ui_connect_automatically_desc),
                    selected = automatic, onClick = { setAutomatic(true) }) },
                { m -> RadioRow(m, stringResource(R.string.ui_connect_when_last), stringResource(R.string.ui_connect_when_last_desc),
                    selected = !automatic, onClick = { setAutomatic(false) }) },
            )
        )

        SectionHeader(stringResource(R.string.settings_notifications))
        SettingsGroup(
            rows = arrayOf(
                if (XposedState.systemUiScopeEnabled) { m -> SwitchRow(m, stringResource(R.string.ui_shade_card), stringResource(R.string.ui_shade_card_desc),
                    checked = shadeCard, icon = R.drawable.ic_ui_notifications, tone = LeadTone.Secondary,
                    onCheckedChange = {
                        shadeCard = it
                        prefs.edit { putBoolean("shade_card", it) }
                        ServiceManager.getService()?.updateNotificationContent(true)
                    }) } else null,
                { m -> SwitchRow(m, stringResource(R.string.ui_connection_notifications), stringResource(R.string.ui_connection_notifications_desc),
                    checked = connectionNotifications, icon = R.drawable.ic_ui_bluetooth_connected, tone = LeadTone.Secondary,
                    onCheckedChange = {
                        connectionNotifications = it
                        prefs.edit { putBoolean("connection_notifications", it) }
                    }) },
            )
        )

        if (state.connectionSuccessful) {
            SectionHeader(stringResource(R.string.conversational_awareness))
            SettingsGroup(
                rows = arrayOf({ m ->
                    Column(m.padding(horizontal = 20.dp, vertical = 16.dp)) {
                        Text(stringResource(R.string.conversational_awareness_volume), style = MaterialTheme.typography.bodyLarge)
                        Slider(
                            value = state.conversationalAwarenessVolume,
                            onValueChange = viewModel::setConversationalAwarenessVolume,
                            valueRange = 10f..85f
                        )
                        Text(
                            "${state.conversationalAwarenessVolume.toInt()}%",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant
                        )
                    }
                })
            )
        }

        val restartBluetoothText = stringResource(R.string.found_offset_restart_bluetooth)
        SectionHeader(stringResource(R.string.advanced_options))
        SettingsGroup(
            rows = arrayOf(
                if (state.connectionSuccessful) { m -> SwitchRow(m, stringResource(R.string.use_alternate_head_tracking_packets),
                    stringResource(R.string.use_alternate_head_tracking_packets_description),
                    checked = state.useAlternateHeadTrackingPackets,
                    onCheckedChange = viewModel::setUseAlternateHeadTrackingPackets) } else null,
                if (XposedState.isAvailable && XposedState.bluetoothScopeEnabled) { m -> SwitchRow(m,
                    stringResource(R.string.act_as_an_apple_device),
                    stringResource(R.string.act_as_an_apple_device_description),
                    checked = state.vendorIdHook,
                    onCheckedChange = { enabled ->
                        Toast.makeText(context, restartBluetoothText, Toast.LENGTH_SHORT).show()
                        viewModel.setVendorIdHook(enabled)
                    }) } else null,
            )
        )

        SectionHeader(stringResource(R.string.settings_help))
        SettingsGroup(
            rows = arrayOf(
                { m -> SettingsRow(m, stringResource(R.string.troubleshooting), stringResource(R.string.settings_troubleshooting_desc),
                    icon = R.drawable.ic_ui_build, tone = LeadTone.Secondary, onClick = navigateToTroubleshooting) },
            )
        )

        SectionHeader(stringResource(R.string.settings_about))
        SettingsGroup(
            rows = arrayOf(
                { m -> SettingsRow(m, stringResource(R.string.version), "${BuildConfig.VERSION_NAME} · ${stringResource(R.string.what_s_new)}",
                    icon = R.drawable.ic_ui_info, tone = LeadTone.Secondary, onClick = navigateToReleaseNotesScreen) },
                { m -> SettingsRow(m, stringResource(R.string.settings_based_on), stringResource(R.string.settings_based_on_desc),
                    icon = R.drawable.ic_ui_history, tone = LeadTone.Secondary, onClick = { open(UPSTREAM_URL) }) },
                { m -> SettingsRow(m, stringResource(R.string.open_source_licenses), null,
                    onClick = navigateToOpenSourceLicenses) },
            )
        )
        Text(
            stringResource(R.string.settings_device_line, "${Build.MANUFACTURER} ${Build.MODEL}", Build.VERSION.RELEASE),
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.padding(start = 8.dp, top = 16.dp, bottom = 32.dp)
        )
    }
}

