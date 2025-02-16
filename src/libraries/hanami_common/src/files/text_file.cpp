/**
 * @file    text_file.cpp
 *
 * @author     Tobias Anker <tobias.anker@kitsunemimi.moe>
 *
 * @copyright  Apache License Version 2.0
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
#include <hanami_common/files/text_file.h>
#include <hanami_common/functions/file_functions.h>
#include <hanami_common/functions/string_functions.h>

namespace Hanami
{

/**
 * @brief read text from a text-file
 *
 * @param readContent reference to the variable, where the content should be written into
 * @param filePath path the to file
 * @param error reference for error-message output
 *
 * @return true if successful, else false
 */
bool
readFile(std::string& readContent, const std::string& filePath, ErrorContainer& error)
{
    // precheck file-location
    std::filesystem::path pathObj(filePath);
    if (std::filesystem::exists(pathObj) == false) {
        error.addMessage("Path '" + filePath + "' doesn't exist.");
        return false;
    }
    if (std::filesystem::exists(pathObj) == false) {
        error.addMessage("Path '" + filePath + "' is not a regular file.");
        return false;
    }

    std::ifstream inFile;
    inFile.open(filePath);
    if (inFile.is_open() == false) {
        error.addMessage("missing permission to open file \"" + filePath + "\"");
        return false;
    }

    std::stringstream strStream;
    strStream << inFile.rdbuf();
    readContent = strStream.str();

    inFile.close();

    return true;
}

/**
 * @brief write text into a file
 *
 * @param filePath path the to file
 * @param content text which be wirtten into the file
 * @param error reference for error-message output
 * @param force if true, it overwrites the file, if there already exist one (Default: true)
 *
 * @return true, if successful, else false
 */
bool
writeFile(const std::string& filePath,
          const std::string& content,
          ErrorContainer& error,
          const bool force)
{
    // check and create parent-directory, if necessary
    const std::filesystem::path parentDirectory = std::filesystem::path(filePath).parent_path();
    if (std::filesystem::exists(parentDirectory) == false) {
        if (createDirectory(parentDirectory, error) == false) {
            return false;
        }
    }

    // check if exist
    if (std::filesystem::exists(filePath)) {
        // check for directory
        if (std::filesystem::is_directory(filePath)) {
            error.addMessage("failed to write destination of path \""
                             + filePath +
                             "\", because it already exist and it is a directory, "
                             "but must be a file or not existing");
            return false;
        }

        // check for override
        if (force == false) {
            error.addMessage("failed to write destination of path \"" + filePath
                             + "\", because it already exist, but should not be overwrite");
            return false;
        }

        // remove file if force-flag is active
        if (deleteFileOrDir(filePath, error) == false) {
            return false;
        }
    }

    // create new file and write content
    std::ofstream outputFile;
    outputFile.open(filePath);
    if (outputFile.is_open() == false) {
        error.addMessage("missing permission or target-directory to open file \"" + filePath
                         + "\"");
        return false;
    }

    outputFile << content;
    outputFile.flush();
    outputFile.close();

    return true;
}

/**
 * @brief append text to a existing text-file
 *
 * @param filePath path the to file
 * @param newText text which should be append to the file
 * @param error reference for error-message output
 *
 * @return true, if successful, else false
 */
bool
appendText(const std::string& filePath, const std::string& newText, ErrorContainer& error)
{
    // precheck file-location
    std::filesystem::path pathObj(filePath);
    if (std::filesystem::exists(pathObj) == false) {
        error.addMessage("Path '" + filePath + "' doesn't exist.");
        return false;
    }
    if (std::filesystem::exists(pathObj) == false) {
        error.addMessage("Path '" + filePath + "' is not a regular file.");
        return false;
    }

    // open, write and close file again
    std::ofstream outputFile;
    outputFile.open(filePath, std::ios_base::app);
    if (outputFile.is_open() == false) {
        error.addMessage("missing permission or target-directory to open file \"" + filePath
                         + "\"");
        return false;
    }

    outputFile << newText;
    outputFile.flush();
    outputFile.close();

    return true;
}

/**
 * @brief replace a specific line inside a text-file
 *
 * @param filePath path the to file
 * @param lineNumber number of the line inside the file, which should be replaced (beginning with 0)
 * @param newLineContent the new content string for the line, which should be replaced
 * @param error reference for error-message output
 *
 * @return true, if successful, else false
 */
bool
replaceLine(const std::string& filePath,
            const uint32_t lineNumber,
            const std::string& newLineContent,
            ErrorContainer& error)
{
    // precheck file-location
    std::filesystem::path pathObj(filePath);
    if (std::filesystem::exists(pathObj) == false) {
        error.addMessage("Path '" + filePath + "' doesn't exist.");
        return false;
    }
    if (std::filesystem::exists(pathObj) == false) {
        error.addMessage("Path '" + filePath + "' is not a regular file.");
        return false;
    }

    // read file
    std::string fileContent = "";
    bool result = readFile(fileContent, filePath, error);
    if (result == false) {
        return false;
    }

    // split content into a vector of lines
    std::vector<std::string> splitedContent;
    Hanami::splitStringByDelimiter(splitedContent, fileContent, '\n');
    if (splitedContent.size() <= lineNumber) {
        error.addMessage("failed to replace line in file \"" + filePath
                         + "\", because linenumber is too big for the file");
        return false;
    }

    // build new file-content
    splitedContent[lineNumber] = newLineContent;
    std::string newFileContent = "";
    for (uint64_t i = 0; i < splitedContent.size(); i++) {
        if (i != 0) {
            newFileContent.append("\n");
        }
        newFileContent.append(splitedContent.at(i));
    }

    // write file back the new content
    return writeFile(filePath, newFileContent, error, true);
}

/**
 * @brief replace a substring inside the file with another string
 *
 * @param filePath path the to file
 * @param oldContent substring which should be replaced
 * @param newContent new string for the replacement
 * @param error reference for error-message output
 *
 * @return true, if successful, else false
 */
bool
replaceContent(const std::string& filePath,
               const std::string& oldContent,
               const std::string& newContent,
               ErrorContainer& error)
{
    // precheck file-location
    std::filesystem::path pathObj(filePath);
    if (std::filesystem::exists(pathObj) == false) {
        error.addMessage("Path '" + filePath + "' doesn't exist.");
        return false;
    }
    if (std::filesystem::exists(pathObj) == false) {
        error.addMessage("Path '" + filePath + "' is not a regular file.");
        return false;
    }

    // read file
    std::string fileContent = "";
    bool result = readFile(fileContent, filePath, error);
    if (result == false) {
        return false;
    }

    // replace content
    std::string::size_type pos = 0u;
    while ((pos = fileContent.find(oldContent, pos)) != std::string::npos) {
        fileContent.replace(pos, oldContent.length(), newContent);
        pos += newContent.length();
    }

    // write file back the new content
    const bool writeResult = writeFile(filePath, fileContent, error, true);

    return writeResult;
}

}  // namespace Hanami
