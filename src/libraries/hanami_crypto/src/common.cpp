/**
 * @file       common.cpp
 *
 *  @author     Tobias Anker <tobias.anker@kitsunemimi.moe>
 *
 *  @copyright  Apache License Version 2.0
 *
 *      Copyright 2022 Tobias Anker
 *
 *      Licensed under the Apache License, Version 2.0 (the "License");
 *      you may not use this file except in compliance with the License.
 *      You may obtain a copy of the License at
 *
 *          http://www.apache.org/licenses/LICENSE-2.0
 *
 *      Unless required by applicable law or agreed to in writing, software
 *      distributed under the License is distributed on an "AS IS" BASIS,
 *      WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *      See the License for the specific language governing permissions and
 *      limitations under the License.
 */

#include <cryptopp/hex.h>
#include <hanami_common/buffer/data_buffer.h>
#include <hanami_crypto/common.h>

namespace Hanami
{

void
hexEncode(std::string& result, const void* data, const uint64_t dataSize)
{
    CryptoPP::HexEncoder encoder;
    encoder.Attach(new CryptoPP::StringSink(result));
    encoder.Put((CryptoPP::byte*)data, dataSize);
    encoder.MessageEnd();
}

//==================================================================================================

struct Base64Buffer3 {
    uint8_t buffer[3];
};

/**
 * @brief convert value into a Base64 character
 *
 * @param val value to convert
 *
 * @return resulting character
 */
inline char
convertValueToBase64(const char val)
{
    if (val >= 0 && val < 26) {
        return val + 65;
    }

    if (val >= 26 && val < 52) {
        return val + 97 - 26;
    }

    if (val >= 52 && val < 62) {
        return val + 48 - 52;
    }

    if (val == 62) {
        return '+';
    }

    return '/';
}

/**
 * @brief convert one cluster of 3 bytes into a Base64 cluster of 4 characters
 *
 * @param result pointer at the specific position inside the char-arrary for the Base64 result
 * @param buf buffer with 3 Bytes to convert
 * @param count number of bytes in the buffer to convert
 */
inline void
convertToBase64(char* result, const Base64Buffer3& buf, const uint8_t count)
{
    // prepare bufferValue
    uint32_t temp = 0;
    temp |= buf.buffer[0];
    temp = temp << 8;
    temp |= buf.buffer[1];
    temp = temp << 8;
    temp |= buf.buffer[2];

    // convert last byte
    if (count == 3) {
        result[3] = convertValueToBase64(static_cast<char>(temp & 0x3F));
    }

    temp = temp >> 6;

    // convert second byte
    if (count >= 2) {
        result[2] = convertValueToBase64(static_cast<char>(temp & 0x3F));
    }

    // convert first byte
    temp = temp >> 6;
    result[1] = convertValueToBase64(static_cast<char>(temp & 0x3F));
    temp = temp >> 6;
    result[0] = convertValueToBase64(static_cast<char>(temp & 0x3F));
}

/**
 * @brief encode Base64 string
 *
 * @param output reference for the resulting string
 * @param data data to convert into Base64
 * @param dataSize number of bytes to convert
 */
void
encodeBase64(std::string& output, const void* data, const uint64_t dataSize)
{
    // prepare buffer for result
    uint64_t resultSize = dataSize / 3;
    if (dataSize % 3 != 0) {
        resultSize++;
    }
    resultSize *= 4;
    char* result = new char[resultSize];

    // transform input
    const Base64Buffer3* buf = static_cast<const Base64Buffer3*>(data);

    // convert base part
    for (uint64_t i = 0; i < dataSize / 3; i++) {
        convertToBase64(&result[i * 4], buf[i], 3);
    }

    // convert padding
    if (dataSize % 3 == 2) {
        const uint64_t endPos = dataSize / 3;
        Base64Buffer3 tempBuf = buf[endPos];
        tempBuf.buffer[2] = 0;
        convertToBase64(&result[endPos * 4], tempBuf, 2);
        result[resultSize - 1] = '=';
    }
    else if (dataSize % 3 == 1) {
        const uint64_t endPos = dataSize / 3;
        Base64Buffer3 tempBuf = buf[endPos];
        tempBuf.buffer[1] = 0;
        tempBuf.buffer[2] = 0;
        convertToBase64(&result[endPos * 4], tempBuf, 1);
        result[resultSize - 2] = '=';
        result[resultSize - 1] = '=';
    }

    // prepare output and delete buffer
    output = std::string(result, resultSize);
    delete[] result;
}

//==================================================================================================

struct Base64Buffer4 {
    uint8_t buffer[4];
};

/**
 * @brief convert Base64 value into the correct bytes for output
 *
 * @param val value to convert
 *
 * @return value with the bytes
 */
inline uint32_t
convertValueFromBase64(const uint8_t val)
{
    // A - Z
    if (val >= 65 && val < 91) {
        return val - 65;
    }

    // a - z
    if (val >= 97 && val < 123) {
        return val - 71;
    }

    // 0 - 9
    if (val >= 48 && val < 58) {
        return val + 4;
    }

    if (val == '+') {
        return 62;
    }

    return 63;
}

/**
 * @brief convert one cluster of 4 characters of a Base64 cluster of 3 bytes
 *
 * @param result pointer to the output buffer
 * @param buf buffer with 4 Bytes to convert
 * @param count number of bytes in the buffer to convert
 */
inline void
convertFromBase64(Base64Buffer3* result, const Base64Buffer4& buf, const uint8_t count)
{
    // prepare bufferValue
    uint32_t temp = 0;
    temp |= buf.buffer[0];
    temp = temp << 8;
    temp |= buf.buffer[1];
    temp = temp << 8;
    temp |= buf.buffer[2];
    temp = temp << 8;
    temp |= buf.buffer[3];

    uint32_t out = 0;

    out |= convertValueFromBase64((temp & 0xFF000000) >> 24) << 18;
    out |= convertValueFromBase64((temp & 0xFF0000) >> 16) << 12;

    if (count >= 2) {
        out |= convertValueFromBase64((temp & 0xFF00) >> 8) << 6;
    }

    if (count >= 3) {
        out |= convertValueFromBase64(temp & 0xFF);
    }

    result->buffer[2] = out & 0xFF;
    result->buffer[1] = (out & 0xFF00) >> 8;
    result->buffer[0] = (out & 0xFF0000) >> 16;
}

/**
 * @brief decode Base64 string
 *
 * @param result buffer for the output of the result
 * @param input Base64 string to convert
 *
 * @return false, if input has invalid length, else true
 */
bool
decodeBase64(DataBuffer& result, const std::string& input)
{
    // precheck
    if (input.size() == 0 || input.size() % 4 != 0) {
        return false;
    }

    // prepare buffer for result
    const uint64_t bufferSize = (input.size() / 4) * 3;
    allocateBlocks_DataBuffer(result, (bufferSize / 4096) + 1);
    result.usedBufferSize = bufferSize;

    // transform input
    const void* tempPtr = static_cast<const void*>(input.c_str());
    const Base64Buffer4* buf = static_cast<const Base64Buffer4*>(tempPtr);
    Base64Buffer3* output = static_cast<Base64Buffer3*>(result.data);

    // convert base part
    for (uint64_t i = 0; i < input.size() / 4 - 1; i++) {
        convertFromBase64(&output[i], buf[i], 3);
    }

    // convert padding
    const uint64_t lastPos = input.size() / 4 - 1;
    if (input.at(input.size() - 1) == '=' && input.at(input.size() - 2) == '=') {
        result.usedBufferSize -= 2;
        convertFromBase64(&output[lastPos], buf[lastPos], 1);
    }
    else if (input.at(input.size() - 1) == '=') {
        result.usedBufferSize -= 1;
        convertFromBase64(&output[lastPos], buf[lastPos], 2);
    }
    else {
        convertFromBase64(&output[lastPos], buf[lastPos], 3);
    }

    return true;
}

/**
 * @brief decode Base64 string
 *
 * @param result string for the output of the result
 * @param input Base64 string to convert
 *
 * @return false, if input has invalid length, else true
 */
bool
decodeBase64(std::string& result, const std::string& input)
{
    DataBuffer buffer;
    const bool ret = decodeBase64(buffer, input);
    if (ret) {
        result = std::string(static_cast<char*>(buffer.data), buffer.usedBufferSize);
    }
    return ret;
}

/**
 * @brief convert an base64 string into a url compatible base64Url version
 *
 * @param base64 reference to the base64-string to convert
 *
 * @return false, if input is invalid, else false
 */
bool
base64ToBase64Url(std::string& base64)
{
    // precheck
    if (base64.size() < 2) {
        return false;
    }

    // replace characters
    for (uint32_t i = 0; i < base64.size(); i++) {
        if (base64[i] == '+') {
            base64[i] = '-';
        }
        if (base64[i] == '/') {
            base64[i] = '_';
        }
    }

    // remove padding
    if (base64[base64.size() - 2] == '=') {
        base64.resize(base64.size() - 2);
    }
    else if (base64[base64.size() - 1] == '=') {
        base64.resize(base64.size() - 1);
    }

    return true;
}

/**
 * @brief convert an base64Url string into a standard base64 version
 *
 * @param base64Url reference to the base64Url-string to convert
 *
 * @return false, if input is invalid, else false
 */
bool
base64UrlToBase64(std::string& base64Url)
{
    // precheck
    if (base64Url.size() < 2) {
        return false;
    }

    // replace characters
    for (uint32_t i = 0; i < base64Url.size(); i++) {
        if (base64Url[i] == '-') {
            base64Url[i] = '+';
        }
        if (base64Url[i] == '_') {
            base64Url[i] = '/';
        }
    }

    // readd padding
    if (base64Url.size() % 4 == 2) {
        base64Url += "==";
    }
    else if (base64Url.size() % 4 == 3) {
        base64Url += "=";
    }

    return true;
}

//==================================================================================================

}  // namespace Hanami
