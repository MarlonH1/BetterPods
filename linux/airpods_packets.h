// airpods_packets.h
#ifndef AIRPODS_PACKETS_H
#define AIRPODS_PACKETS_H

#include <QByteArray>
#include <QList>
#include <QString>
#include <QPair>
#include <optional>
#include <algorithm>
#include <climits>

#include "enums.h"
#include "BasicControlCommand.hpp"

namespace AirPodsPackets
{
    // Noise Control Mode Packets
    namespace NoiseControl
    {
        using NoiseControlMode = AirpodsTrayApp::Enums::NoiseControlMode;
        static const QByteArray HEADER = ControlCommand::HEADER + 0x0D;
        static const QByteArray OFF = ControlCommand::createCommand(0x0D, 0x01);
        static const QByteArray NOISE_CANCELLATION = ControlCommand::createCommand(0x0D, 0x02);
        static const QByteArray TRANSPARENCY = ControlCommand::createCommand(0x0D, 0x03);
        static const QByteArray ADAPTIVE = ControlCommand::createCommand(0x0D, 0x04);

        static const QByteArray getPacketForMode(AirpodsTrayApp::Enums::NoiseControlMode mode)
        {
            switch (mode)
            {
            case NoiseControlMode::Off:
                return OFF;
            case NoiseControlMode::NoiseCancellation:
                return NOISE_CANCELLATION;
            case NoiseControlMode::Transparency:
                return TRANSPARENCY;
            case NoiseControlMode::Adaptive:
                return ADAPTIVE;
            default:
                return QByteArray();
            }
        }

        inline std::optional<NoiseControlMode> parseMode(const QByteArray &data)
        {
            char mode = ControlCommand::parseActive(data).value_or(CHAR_MAX) - 1;
            if (mode < static_cast<quint8>(NoiseControlMode::MinValue) ||
                mode > static_cast<quint8>(NoiseControlMode::MaxValue))
            {
                return std::nullopt;
            }
            return static_cast<NoiseControlMode>(mode);
        }
    }

    // One Bud ANC Mode
    namespace OneBudANCMode
    {
        using Type = BasicControlCommand<0x1B>;
        static const QByteArray ENABLED = Type::ENABLED;
        static const QByteArray DISABLED = Type::DISABLED;
        static const QByteArray HEADER = Type::HEADER;
        inline std::optional<bool> parseState(const QByteArray &data) { return Type::parseState(data); }
    }

    // Volume Swipe (partial - still needs custom interval function)
    namespace VolumeSwipe
    {
        using Type = BasicControlCommand<0x25>;
        static const QByteArray ENABLED = Type::ENABLED;
        static const QByteArray DISABLED = Type::DISABLED;
        static const QByteArray HEADER = Type::HEADER;
        inline std::optional<bool> parseState(const QByteArray &data) { return Type::parseState(data); }

        // Keep custom interval function
        static QByteArray getIntervalPacket(quint8 interval)
        {
            return ControlCommand::createCommand(0x23, interval);
        }
    }

    // Adaptive Volume Config
    namespace AdaptiveVolume
    {
        using Type = BasicControlCommand<0x26>;
        static const QByteArray ENABLED = Type::ENABLED;
        static const QByteArray DISABLED = Type::DISABLED;
        static const QByteArray HEADER = Type::HEADER;
        inline std::optional<bool> parseState(const QByteArray &data) { return Type::parseState(data); }
    }

    // Conversational Awareness
    namespace ConversationalAwareness
    {
        using Type = BasicControlCommand<0x28>;
        static const QByteArray ENABLED = Type::ENABLED;
        static const QByteArray DISABLED = Type::DISABLED;
        static const QByteArray HEADER = Type::HEADER;
        static const QByteArray DATA_HEADER = QByteArray::fromHex("040004004B00020001");
        inline std::optional<bool> parseState(const QByteArray &data) { return Type::parseState(data); }
    }

    // Hearing Assist
    namespace HearingAssist
    {
        using Type = BasicControlCommand<0x33>;
        static const QByteArray ENABLED = Type::ENABLED;
        static const QByteArray DISABLED = Type::DISABLED;
        static const QByteArray HEADER = Type::HEADER;
        inline std::optional<bool> parseState(const QByteArray &data) { return Type::parseState(data); }
    }

    // Hearing Aid
    namespace HearingAid
    {
        static const QByteArray HEADER = ControlCommand::HEADER + static_cast<char>(0x2C);
        static const QByteArray ENABLED = ControlCommand::createCommand(0x2C, 0x01, 0x01);
        static const QByteArray DISABLED = ControlCommand::createCommand(0x2C, 0x02, 0x02);

        inline std::optional<bool> parseState(const QByteArray &data)
        {
            if (!data.startsWith(HEADER) || data.size() < HEADER.size() + 2)
                return std::nullopt;

            QByteArray value = data.mid(HEADER.size(), 2);
            if (value.size() != 2)
                return std::nullopt;

            char b1 = value.at(0);
            char b2 = value.at(1);

            if (b1 == 0x01 && b2 == 0x01)
                return true;
            if (b1 == 0x02 || b2 == 0x02)
                return false;

            return std::nullopt;
        }
    }

    // Allow Off Option
    namespace AllowOffOption
    {
        using Type = BasicControlCommand<0x34>;
        static const QByteArray ENABLED = Type::ENABLED;
        static const QByteArray DISABLED = Type::DISABLED;
        static const QByteArray HEADER = Type::HEADER;
        inline std::optional<bool> parseState(const QByteArray &data) { return Type::parseState(data); }
    }

    // Connection Packets
    namespace Connection
    {
        static const QByteArray HANDSHAKE = QByteArray::fromHex("00000400010002000000000000000000");
        static const QByteArray SET_SPECIFIC_FEATURES = QByteArray::fromHex("040004004d00d700000000000000");
        static const QByteArray REQUEST_NOTIFICATIONS = QByteArray::fromHex("040004000f00ffffffffff");
        static const QByteArray AIRPODS_DISCONNECTED = QByteArray::fromHex("00010000");
    }

    // Phone Communication Packets
    namespace Phone
    {
        static const QByteArray NOTIFICATION = QByteArray::fromHex("00040001");
        static const QByteArray CONNECTED = QByteArray::fromHex("00010001");
        static const QByteArray DISCONNECTED = QByteArray::fromHex("00010000");
        static const QByteArray STATUS_REQUEST = QByteArray::fromHex("00020003");
        static const QByteArray DISCONNECT_REQUEST = QByteArray::fromHex("00020000");
    }

    // Adaptive Noise Packets
    namespace AdaptiveNoise
    {
        const QByteArray HEADER = QByteArray::fromHex("0400040009002E");

        inline QByteArray getPacket(int level)
        {
            return HEADER + static_cast<char>(level) + QByteArray::fromHex("000000");
        }
    }

    namespace Rename
    {
        static QByteArray getPacket(const QString &newName)
        {
            QByteArray nameBytes = newName.toUtf8();                   // Convert name to UTF-8
            quint8 size = static_cast<char>(nameBytes.size());         // Name length (1 byte)
            QByteArray packet = QByteArray::fromHex("040004001A0001"); // Header
            packet.append(size);                                       // Append size byte
            packet.append('\0');                                       // Append null byte
            packet.append(nameBytes);                                  // Append name bytes
            return packet;
        }
    }

    namespace MagicPairing {
        static const QByteArray REQUEST_MAGIC_CLOUD_KEYS = QByteArray::fromHex("0400040030000500");
        static const QByteArray MAGIC_CLOUD_KEYS_HEADER = QByteArray::fromHex("04000400310002");

        struct MagicCloudKeys {
            QByteArray magicAccIRK;      // 16 bytes
            QByteArray magicAccEncKey;    // 16 bytes
        };

        inline MagicCloudKeys parseMagicCloudKeysPacket(const QByteArray &data)
        {
            MagicCloudKeys keys;

            if (data.size() < 47 || !data.startsWith(MAGIC_CLOUD_KEYS_HEADER))
            {
                return keys;
            }

            int index = MAGIC_CLOUD_KEYS_HEADER.size();

            // First TLV block (MagicAccIRK)
            if (static_cast<quint8>(data.at(index)) != 0x01)
                return keys;
            index += 1;

            quint16 len1 = (static_cast<quint8>(data.at(index)) << 8) | static_cast<quint8>(data.at(index + 1));
            if (len1 != 16)
                return keys;
            index += 3; // Skip length (2 bytes) and reserved byte (1 byte)

            keys.magicAccIRK = data.mid(index, 16);
            index += 16;

            // Second TLV block (MagicAccEncKey)
            if (static_cast<quint8>(data.at(index)) != 0x04)
                return keys;
            index += 1;

            quint16 len2 = (static_cast<quint8>(data.at(index)) << 8) | static_cast<quint8>(data.at(index + 1));
            if (len2 != 16)
                return keys;
            index += 3; // Skip length (2 bytes) and reserved byte (1 byte)

            keys.magicAccEncKey = data.mid(index, 16);

            return keys;
        }
    }

    namespace OwnsConnection
    {
        using Type = BasicControlCommand<0x06>;
        static const QByteArray ENABLED = Type::ENABLED;
        static const QByteArray RELEASED = ControlCommand::createCommand(0x06, 0x00);
        static const QByteArray HEADER = Type::HEADER;
        inline bool parseOwns(const QByteArray &data) { return Type::getValue(data) == 0x01; }
    }

    namespace SmartRouting
    {
        static const QByteArray HEADER = QByteArray::fromHex("040004001000");
        static const QByteArray CONNECTED_DEVICES_HEADER = QByteArray::fromHex("040004002e");
        static const QByteArray AUDIO_SOURCE_HEADER = QByteArray::fromHex("040004000e");
        static const QByteArray RESPONSE_HEADER = QByteArray::fromHex("040004001100");
        static const char AUDIO_SOURCE_NONE = 0x00;

        struct AudioSource {
            QString mac;
            char type;
        };

        inline QString formatMac(const QByteArray &mac) { return QString::fromLatin1(mac.toHex(':').toUpper()); }

        namespace Opack
        {
            inline QByteArray string(const QByteArray &value)
            {
                if (value.size() <= 32)
                    return char(0x40 + value.size()) + value;
                return char(0x61) + QByteArray(1, char(value.size())) + value;
            }

            inline QByteArray integer(int value)
            {
                if (value >= 0 && value <= 39)
                    return QByteArray(1, char(0x08 + value));
                if (value >= -128 && value <= 127)
                    return char(0x30) + QByteArray(1, char(value));
                return char(0x31) + QByteArray(1, char(value & 0xFF)) + char((value >> 8) & 0xFF);
            }

            inline QByteArray boolean(bool value) { return QByteArray(1, value ? 0x01 : 0x02); }

            inline QByteArray dictionary(const QList<QPair<QByteArray, QByteArray>> &entries)
            {
                QByteArray data(1, char(0xE0 + entries.size()));
                for (const auto &[key, value] : entries)
                    data += string(key) + value;
                return data;
            }
        }

        inline QByteArray createPacket(QByteArray target, const QByteArray &body)
        {
            std::reverse(target.begin(), target.end());
            const int length = body.size() + 1;
            return HEADER + target + char(length & 0xFF) + char(length >> 8) + char(0x01) + body;
        }

        inline QByteArray createMediaInformation(const QString &selfMac, const QByteArray &target, bool playing)
        {
            using namespace Opack;
            return createPacket(target, dictionary({
                {"playingApp", string(playing ? "com.google.ios.youtube" : "Unknown")},
                {"hostStreamingState", string(playing ? "YES" : "NO")},
                {"btAddress", string(selfMac.toLatin1())},
                {"btName", string("Android")},
                {"otherDeviceAudioCategory", integer(playing ? 301 : 100)},
            }));
        }

        inline QByteArray createShowUI(const QByteArray &target)
        {
            using namespace Opack;
            return createPacket(target, dictionary({
                {"SmartRoutingKeyShowNearbyUI", boolean(true)},
                {"localscore", integer(301)},
                {"reason", string("Hijackv2")},
                {"audioRoutingScore", integer(301)},
                {"audioRoutingSetOwnershipToFalse", boolean(true)},
                {"remotescore", integer(100)},
            }));
        }

        inline QByteArray createHijackRequest(const QByteArray &target)
        {
            using namespace Opack;
            return createPacket(target, dictionary({
                {"localscore", integer(301)},
                {"reason", string("Hijackv2")},
                {"audioRoutingScore", integer(301)},
                {"audioRoutingSetOwnershipToFalse", boolean(true)},
                {"remotescore", integer(100)},
            }));
        }

        inline QByteArray createAddTiPiDevice(const QString &selfMac, const QByteArray &target)
        {
            using namespace Opack;
            return createPacket(target, dictionary({
                {"idleTime", integer(0)},
                {"newTipi", boolean(true)},
                {"btAddress", string(selfMac.toLatin1())},
                {"btName", string("Android")},
                {"nearbyAudioScore", integer(6)},
            }));
        }

        inline QList<QByteArray> parseConnectedDevices(const QByteArray &data)
        {
            QList<QByteArray> devices;
            if (data.size() < 9)
                return devices;
            for (int i = 0, offset = 9; i < data.at(8) && offset + 8 <= data.size(); i++, offset += 8)
                devices.append(data.mid(offset, 6));
            return devices;
        }

        inline std::optional<AudioSource> parseAudioSource(const QByteArray &data)
        {
            if (data.size() < 13)
                return std::nullopt;
            QByteArray mac = data.mid(6, 6);
            std::reverse(mac.begin(), mac.end());
            return AudioSource{formatMac(mac), data.at(12)};
        }
    }

    // Parsing Headers
    namespace Parse
    {
        static const QByteArray EAR_DETECTION = QByteArray::fromHex("040004000600");
        static const QByteArray BATTERY_STATUS = QByteArray::fromHex("040004000400");
        static const QByteArray METADATA = QByteArray::fromHex("040004001d");
        static const QByteArray HANDSHAKE_ACK = QByteArray::fromHex("01000400");
        static const QByteArray FEATURES_ACK = QByteArray::fromHex("040004002b00"); // Note: Only tested with airpods pro 2
    }
}

#endif // AIRPODS_PACKETS_H
