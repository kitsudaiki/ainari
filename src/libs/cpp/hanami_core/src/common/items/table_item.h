/**
 * @file       table_items.cpp
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

#ifndef TABLE_ITEM_H
#define TABLE_ITEM_H

#include <stdint.h>

#include <nlohmann/json.hpp>
#include <vector>

using json = nlohmann::json;

namespace Hanami
{

class TableItem
{
   public:
    TableItem();
    TableItem(const TableItem& other);
    TableItem(const json& body, const json& header = json::array());
    ~TableItem();
    TableItem& operator=(const TableItem& other);

    void clearTable();

    // column
    bool addColumn(const std::string& internalName, const std::string& shownName = "");
    bool renameColume(const std::string& internalName, const std::string& newShownName);
    bool deleteColumn(const uint64_t x);
    bool deleteColumn(const std::string& internalName);

    // row
    bool addRow(const json& rowContent);
    bool addRowVec(const std::vector<std::string> rowContent);
    bool deleteRow(const uint64_t y);

    // cell
    bool setCell(const uint32_t column, const uint32_t row, const std::string& newValue);
    const std::string getCell(const uint32_t column, const uint32_t row);
    bool deleteCell(const uint32_t column, const uint32_t row);

    // size
    uint64_t getNumberOfColums();
    uint64_t getNumberOfRows();

    // getter complete
    json getHeader() const;
    json getInnerHeader() const;
    json getBody() const;
    json stealContent();
    json getRow(const uint32_t row) const;

    // output
    const std::string toString(const uint32_t maxColumnWidth = 500,
                               const bool withoutHeader = false);
    const std::string toJsonString();

   private:
    json m_body;
    json m_header;

    // internal typedefs to make cleaner code
    typedef std::vector<std::string> TableCell;
    typedef std::vector<TableCell> TableRow;
    typedef std::vector<TableRow> TableBodyAll;

    // helper functions for the output
    const std::vector<std::string> getInnerName();
    const std::string getLimitLine(const std::vector<uint64_t>& sizes, const bool bigLine = false);

    // content-converter for easier output-handline
    void convertCellForOutput(TableCell& convertedCell,
                              const std::string& cellContent,
                              uint64_t& width,
                              const uint32_t maxColumnWidth);
    void convertHeaderForOutput(TableRow& convertedHeader,
                                std::vector<uint64_t>& xSizes,
                                const uint32_t maxColumnWidth);
    void convertBodyForOutput(TableBodyAll& convertedBody,
                              std::vector<uint64_t>& xSizes,
                              std::vector<uint64_t>& ySizes,
                              const uint32_t maxColumnWidth);

    // output of single lines of the output
    const std::string printHeaderLine(const std::vector<uint64_t>& xSizes);
    const std::string printBodyLine(TableRow& rowContent,
                                    const std::vector<uint64_t>& xSizes,
                                    const uint64_t rowHeigh);
    const std::string printHeaderBodyLine(TableRow& headerContent,
                                          TableRow& rowContent,
                                          const std::vector<uint64_t>& xSizes,
                                          const uint64_t rowHeigh,
                                          const uint64_t y);

    // final output of the two different versions
    const std::string printNormalTable(TableBodyAll& convertedBody,
                                       std::vector<uint64_t>& xSizes,
                                       std::vector<uint64_t>& ySizes,
                                       const bool withoutHeader);
};

}  // namespace Hanami

#endif  // TABLE_ITEM_H
