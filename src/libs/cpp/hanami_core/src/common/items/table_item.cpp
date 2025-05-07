/**
 * @file       table_items.h
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

#include <src/common/functions/string_functions.h>
#include <src/common/items/table_item.h>

namespace Hanami
{

/**
 * @brief constructor
 */
TableItem::TableItem()
{
    m_header = json::array();
    m_body = json::array();
}

/**
 * @brief copy-constructor
 */
TableItem::TableItem(const TableItem& other)
{
    m_body = other.m_body;
    m_header = other.m_header;
}

/**
 * @brief create a table from predefined values
 *
 * @param body body-content as data-array-item
 * @param header header-content as data-array-item
 */
TableItem::TableItem(const json& body, const json& header)
{
    m_header = header;
    m_body = body;
}

/**
 * @brief destructor
 */
TableItem::~TableItem() {}

/**
 * @brief assignment-constructor
 */
TableItem&
TableItem::operator=(const TableItem& other)
{
    // check for self-assignment
    if (&other == this) {
        return *this;
    }

    m_body = other.m_body;
    m_header = other.m_header;

    return *this;
}

/**
 * @brief erase the whole content of the table
 */
void
TableItem::clearTable()
{
    m_header = json::array();
    m_body = json::array();
}

/**
 * @brief add a new column to the header of the table
 *
 * @param internalName name for internal indentification of the columns inside the body
 * @param shownName name which is shown in the string-output of the table
 *                  if leaved blank, the name is set equal to the internal-name
 *
 * @return should return true everytime
 */
bool
TableItem::addColumn(const std::string& internalName, const std::string& shownName)
{
    json obj = json::object();

    obj["inner"] = internalName;
    if (shownName != "") {
        obj["outer"] = shownName;
    }
    else {
        obj["outer"] = internalName;
    }

    m_header.push_back(obj);

    return true;
}

/**
 * @brief rename a column in the header of the table
 *
 * @param internalName name for internal indentification of the columns inside the body
 * @param newShownName new name for string output
 *
 * @return false if internal-name doesn't exist, else true
 */
bool
TableItem::renameColume(const std::string& internalName, const std::string& newShownName)
{
    const uint64_t size = m_header.size();

    for (uint64_t x = 0; x < size; x++) {
        if (m_header[x]["inner"] == internalName) {
            m_header[x]["outer"] = newShownName;
            return true;
        }
    }

    return false;
}

/**
 * @brief delelete a colume from the table
 *
 * @param x column-position
 *
 * @return false if column-position is too high, else true
 */
bool
TableItem::deleteColumn(const uint64_t x)
{
    // precheck
    if (x >= m_header.size()) {
        return false;
    }

    // remove colume from header
    m_header.erase(x);

    // remove data of the column
    const uint64_t size = m_body.size();
    for (uint64_t y = 0; y < size; y++) {
        m_body[y].erase(x);
    }

    return true;
}

/**
 * @brief delelete a colume from the table
 *
 * @param internalName internal name of the column
 *
 * @return false if internal name doesn't exist, else true
 */
bool
TableItem::deleteColumn(const std::string& internalName)
{
    const uint64_t size = m_header.size();

    // search in header
    for (uint64_t x = 0; x < size; x++) {
        if (m_header[x]["inner"] == internalName) {
            return deleteColumn(x);
        }
    }

    return false;
}

/**
 * @brief TableItem::addRow
 *
 * @param rowContent new table-row
 *
 * @return false, if the new row has not the correct length to fit into the table, else true
 */
bool
TableItem::addRow(const json& rowContent)
{
    // check if the new row has the correct length
    if (rowContent.size() != getNumberOfColums()) {
        return false;
    }

    m_body.push_back(rowContent);

    return true;
}

/**
 * @brief add a new row to the table
 *
 * @param rowContent vector of string for the content of the new row
 *
 * @return false, if the new row has not the correct length to fit into the table, else true
 */
bool
TableItem::addRowVec(const std::vector<std::string> rowContent)
{
    // check if the new row has the correct length
    if (rowContent.size() != getNumberOfColums()) {
        return false;
    }

    // add new row content to the table
    json obj = json::array();
    for (uint64_t x = 0; x < rowContent.size(); x++) {
        obj.push_back(rowContent.at(x));
    }

    m_body.push_back(obj);

    return true;
}

/**
 * @brief delete a row from the table
 *
 * @param y row-position
 *
 * @return false if row-position is too high, else true
 */
bool
TableItem::deleteRow(const uint64_t y)
{
    // precheck
    if (y >= m_body.size()) {
        return false;
    }

    m_body.erase(y);

    return true;
}

/**
 * @brief set the content of a specific cell inside the table
 *
 * @param column x-position of the cell within the table
 * @param row y-position of the cell within the table
 * @param newValue new cell-value as string
 *
 * @return false if x or y is too hight, esle true
 */
bool
TableItem::setCell(const uint32_t column, const uint32_t row, const std::string& newValue)
{
    // precheck
    if (column >= m_header.size() || row >= m_body.size()) {
        return false;
    }

    // set new value
    m_body[row][column] = newValue;

    return true;
}

/**
 * @brief request the content of a specific cell of the table
 *
 * @param column x-position of the cell within the table
 * @param row y-position of the cell within the table
 *
 * @return content of the cell as string or empty-string if cell is not set or exist
 *         and also empty string, if x or y is too hight
 */
const std::string
TableItem::getCell(const uint32_t column, const uint32_t row)
{
    // precheck
    if (column >= m_header.size() || row >= m_body.size()) {
        return "";
    }

    json value = m_body[row][column];
    if (value.is_null()) {
        return "";
    }

    if (value.is_string()) {
        return value;
    }
    else {
        return value.dump();
    }
}

/**
 * @brief delete a spcific cell from the table
 *
 * @param column x-position of the cell within the table
 * @param row y-position of the cell within the table
 *
 * @return false if cell-content is already deleted or if x or y is too hight, else true
 */
bool
TableItem::deleteCell(const uint32_t column, const uint32_t row)
{
    // precheck
    if (column >= m_header.size() || row >= m_body.size()) {
        return false;
    }

    m_body[row][column] = nullptr;

    return true;
}

/**
 * @brief request number of columns of the table
 *
 * @return number of columns
 */
uint64_t
TableItem::getNumberOfColums()
{
    return m_header.size();
}

/**
 * @brief request number of rows of the table
 *
 * @return number of rows
 */
uint64_t
TableItem::getNumberOfRows()
{
    return m_body.size();
}

/**
 * @brief get table header
 *
 * @return copy of table-header
 */
json
TableItem::getHeader() const
{
    return m_header;
}

/**
 * @brief get table header, but only the inner names
 *
 * @return copy of table-header
 */
json
TableItem::getInnerHeader() const
{
    json newArray = json::array();
    for (uint32_t i = 0; i < m_header.size(); i++) {
        newArray.push_back(m_header[i]["inner"]);
    }

    return newArray;
}

/**
 * @brief get table body
 *
 * @return copy of table-body
 */
json
TableItem::getBody() const
{
    return m_body;
}

/**
 * @brief steal content of the table
 *
 * @return stolen content
 */
json
TableItem::stealContent()
{
    json content = json::object();
    content["header"] = m_header;
    content["body"] = m_body;
    return content;
}

/**
 * @brief get a specific row of the table
 *
 * @param row number of requested row
 *
 * @return copy of the requested row
 */
json
TableItem::getRow(const uint32_t row) const
{
    // check if out of range
    if (row >= m_body.size()) {
        return nullptr;
    }

    return m_body[row];
}

/**
 * @brief converts the table-content into a string
 *
 * @param maxColumnWidth maximum width of a column in number of characters
 * @param withoutHeader if true, the header line of the table will not be printed
 *
 * @return table as string
 */
const std::string
TableItem::toString(const uint32_t maxColumnWidth, const bool withoutHeader)
{
    // init data-handling values
    std::vector<uint64_t> xSizes(getNumberOfColums(), 0);
    std::vector<uint64_t> ySizes(getNumberOfRows(), 0);
    TableBodyAll convertedBody;
    TableRow convertedHeader;

    // converts the table into a better format for the output
    // and get the maximum values of the columns and rows
    convertHeaderForOutput(convertedHeader, xSizes, maxColumnWidth);
    convertBodyForOutput(convertedBody, xSizes, ySizes, maxColumnWidth);

    // print as normal table
    return printNormalTable(convertedBody, xSizes, ySizes, withoutHeader);
}

/**
 * @brief converts header and body of the table into one single json-formated string without indent
 *
 * @return json-string
 */
const std::string
TableItem::toJsonString()
{
    std::string result = "{ header: ";
    result.append(m_header.dump());
    result.append(", body: ");
    result.append(m_body.dump());
    result.append("}");
    Hanami::replaceSubstring(result, "\n", "\\n");
    return result;
}

/**
 * @brief output of the content as classic table.
 *
 * @param convertedBody content of the body in converted form
 * @param xSizes target of the x-size values
 * @param ySizes target of the y-size values
 * @param withoutHeader if true, the header line of the table will not be printed
 *
 * @return table as string
 */
const std::string
TableItem::printNormalTable(TableBodyAll& convertedBody,
                            std::vector<uint64_t>& xSizes,
                            std::vector<uint64_t>& ySizes,
                            const bool withoutHeader)
{
    // create separator-line
    const std::string normalSeparator = getLimitLine(xSizes);
    std::string result = "";

    // print table-header
    result.append(normalSeparator);
    if (withoutHeader == false) {
        result.append(printHeaderLine(xSizes));
        result.append(getLimitLine(xSizes, true));
    }

    // print table body
    for (uint64_t y = 0; y < getNumberOfRows(); y++) {
        result.append(printBodyLine(convertedBody.at(y), xSizes, ySizes.at(y)));
        result.append(getLimitLine(xSizes));
    }

    return result;
}

/**
 * @brief get all internal column-names
 *
 * @return list with all internal names
 */
const std::vector<std::string>
TableItem::getInnerName()
{
    std::vector<std::string> result;
    result.reserve(m_header.size());

    for (uint64_t x = 0; x < getNumberOfColums(); x++) {
        result.push_back(m_header[x]["inner"]);
    }

    return result;
}

/**
 * @brief finialize a cell of the table for output, by spliting it into multiple lines
 *        if necessary.
 *
 * @param convertedCell target of the result of the convert
 * @param cellContent cell-content as string
 * @param width width of the current column
 * @param maxColumnWidth maximum with of a single column in number of characters
 */
void
TableItem::convertCellForOutput(TableCell& convertedCell,
                                const std::string& cellContent,
                                uint64_t& width,
                                const uint32_t maxColumnWidth)
{
    splitStringByDelimiter(convertedCell, cellContent, '\n');
    for (uint32_t line = 0; line < convertedCell.size(); line++) {
        if (convertedCell.at(line).size() > maxColumnWidth) {
            std::vector<std::string> sub;
            splitStringByLength(sub, convertedCell.at(line), maxColumnWidth);

            // delete old entry and replace it with the splitted content
            convertedCell.erase(convertedCell.begin() + line);
            convertedCell.insert(convertedCell.begin() + line, sub.begin(), sub.end());
        }

        // check for a new maximum of the column-width
        if (width < convertedCell.at(line).size()) {
            width = convertedCell.at(line).size();
        }
    }
}

/**
 * @brief converts the header of the table from a data-item-tree into a string-lists
 *
 * @param convertedHeader target of the result of the convert
 * @param xSizes target of the x-size values
 * @param maxColumnWidth maximum with of a single column in number of characters
 */
void
TableItem::convertHeaderForOutput(TableRow& convertedHeader,
                                  std::vector<uint64_t>& xSizes,
                                  const uint32_t maxColumnWidth)
{
    for (uint64_t x = 0; x < xSizes.size(); x++) {
        std::string cellContent = "";

        // get value at requested position
        json value = m_header[x]["outer"];
        if (value.is_null() == false) {
            cellContent = value;
        }

        // split cell content
        TableCell splittedCellContent;
        convertCellForOutput(splittedCellContent, cellContent, xSizes[x], maxColumnWidth);

        convertedHeader.push_back(splittedCellContent);
    }
}

/**
 * @brief converts the body of the table from a data-item-tree into a string-lists
 *
 * @param convertedBody target of the result of the convert
 * @param xSizes target of the x-size values
 * @param ySizes target of the y-size values
 * @param maxColumnWidth maximum with of a single column in number of characters
 */
void
TableItem::convertBodyForOutput(TableBodyAll& convertedBody,
                                std::vector<uint64_t>& xSizes,
                                std::vector<uint64_t>& ySizes,
                                const uint32_t maxColumnWidth)
{
    for (uint64_t y = 0; y < getNumberOfRows(); y++) {
        convertedBody.push_back(TableRow());

        for (uint64_t x = 0; x < getNumberOfColums(); x++) {
            std::string cellContent = "";
            const json tempVal = m_body[y][x];
            if (tempVal.is_null()) {
                cellContent = "";
            }
            else if (tempVal.is_string()) {
                cellContent = tempVal;
            }
            else {
                cellContent = tempVal.dump();
            }

            // split cell content
            TableCell splittedCellContent;
            convertCellForOutput(splittedCellContent, cellContent, xSizes[x], maxColumnWidth);

            // check for a new maximum of the row-height
            if (ySizes.at(y) < splittedCellContent.size()) {
                ySizes[y] = splittedCellContent.size();
            }

            convertedBody.at(y).push_back(splittedCellContent);
        }
    }
}

/**
 * @brief create separator-line for the table
 *
 * @param sizes list with all width-values for printing placeholder
 * @param bigLine if true, it use '=' instead of '-' for the lines (default = false)
 *
 * @return separator-line as string
 */
const std::string
TableItem::getLimitLine(const std::vector<uint64_t>& sizes, const bool bigLine)
{
    std::string output = "";

    // set line type
    char lineSegment = '-';
    if (bigLine) {
        lineSegment = '=';
    }

    // create line
    for (uint64_t i = 0; i < sizes.size(); i++) {
        output.append("+");
        output.append(std::string(sizes.at(i) + 2, lineSegment));
    }
    output.append("+\n");

    return output;
}

/**
 * @brief convert the header of the table into a string
 *
 * @param xSizes list with all width-values for printing placeholder
 *
 * @return sting with the content of the header for output
 */
const std::string
TableItem::printHeaderLine(const std::vector<uint64_t>& xSizes)
{
    std::string output = "";

    for (uint64_t i = 0; i < xSizes.size(); i++) {
        const std::string value = m_header[i]["outer"];
        output.append("| ");
        output.append(value);
        output.append(std::string(xSizes.at(i) - value.size(), ' '));
        output.append(" ");
    }

    output.append("|\n");

    return output;
}

/**
 * @brief converts a row of the table into a string
 *
 * @param rowContent one row of the converted content of the table-body
 * @param xSizes list with all width-values for printing placeholder
 * @param rowHeight hight-value for printing placeholder
 *
 * @return sting with the content of a row for output
 */
const std::string
TableItem::printBodyLine(TableRow& rowContent,
                         const std::vector<uint64_t>& xSizes,
                         const uint64_t rowHeight)
{
    std::string output = "";

    // create output string for all lines of one table-row
    for (uint64_t line = 0; line < rowHeight; line++) {
        // print row line by line
        for (uint64_t i = 0; i < xSizes.size(); i++) {
            std::string singleCellLine = "";
            if (rowContent.at(i).size() > line) {
                singleCellLine = rowContent.at(i).at(line);
            }

            // create string for one line of one cell
            output.append("| ");
            output.append(singleCellLine);
            output.append(std::string(xSizes.at(i) - singleCellLine.size(), ' '));
            output.append(" ");
        }

        output.append("|\n");
    }

    return output;
}

/**
 * @brief converts a row of the table into a string
 *
 * @param convertedHeader target of the result of the convert
 * @param rowContent one row of the converted content of the table-body
 * @param xSizes list with all width-values for printing placeholder
 * @param rowHeigh number of lines of the table row
 * @param y number of the table row
 *
 * @return sting with the content of a row for output
 */
const std::string
TableItem::printHeaderBodyLine(TableItem::TableRow& headerContent,
                               TableItem::TableRow& rowContent,
                               const std::vector<uint64_t>& xSizes,
                               const uint64_t rowHeigh,
                               const uint64_t y)
{
    std::string output = "";

    // create output string for all lines of one table-row
    for (uint64_t line = 0; line < rowHeigh; line++) {
        std::string singleCellLine;

        // get line-content for the left side
        singleCellLine = "";
        if (headerContent.at(y).size() > line) {
            singleCellLine = headerContent.at(y).at(line);
        }

        // print left side
        output.append("| ");
        output.append(singleCellLine);
        output.append(std::string(xSizes.at(y) - singleCellLine.size(), ' '));
        output.append(" ");

        // get line-content for the right side
        singleCellLine = "";
        if (rowContent.at(y).size() > line) {
            singleCellLine = rowContent.at(y).at(line);
        }

        // print right side
        output.append("| ");
        output.append(singleCellLine);
        output.append(std::string(xSizes.at(y) - singleCellLine.size(), ' '));
        output.append(" ");

        output.append("|\n");
    }

    return output;
}

}  // namespace Hanami
