/**
 * @file    ini_parser.y
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

%skeleton "lalr1.cc"

%defines

//requires 3.2 to avoid the creation of the stack.hh
%require "3.8.2"
%define api.parser.class {IniParser}

%define api.prefix {ini}
%define api.namespace {Hanami}
%define api.token.constructor
%define api.value.type variant
%define parse.assert

%code requires
{
#include <string>
#include <map>
#include <utility>
#include <iostream>
#include <vector>
#include <nlohmann/json.hpp>

using json = nlohmann::json;

namespace Hanami
{
class IniParserInterface;
}
}

// The parsing context.
%param { Hanami::IniParserInterface& driver }

%locations

%code
{
#include <ini_parsing/ini_parser_interface.h>
# undef YY_DECL
# define YY_DECL \
    Hanami::IniParser::symbol_type inilex (Hanami::IniParserInterface& driver)
YY_DECL;
}

// Token
%define api.token.prefix {Ini_}
%token
    END  0  "end of file"
    LINEBREAK "lbreak"
    BOOL_TRUE  "true"
    BOOL_FALSE "false"
    EQUAL "="
    BRACKOPEN "["
    BRACKCLOSE "]"
    COMMA ","
;

%token <std::string> IDENTIFIER "identifier"
%token <std::string> STRING "string"
%token <std::string> STRING_PLN "string_pln"
%token <long> NUMBER "number"
%token <double> FLOAT "float"

%type <json> grouplist
%type <std::string> groupheader
%type <json> itemlist
%type <json> itemValue
%type <json> identifierlist

%%
%start startpoint;

startpoint:
    grouplist
    {
        driver.setOutput($1);
    }

grouplist:
    grouplist groupheader itemlist
    {
        $1[$2] = $3;
        $$ = $1;
    }
|
    grouplist groupheader
    {
        $1[$2] = json::object();
        $$ = $1;
    }
|
    groupheader itemlist
    {
        json newMap = json::object();
        newMap[$1] = $2;
        $$ = newMap;
    }
|
    groupheader
    {
        json newMap = json::object();
        newMap[$1] = json::object();
        $$ = newMap;
    }

groupheader:
    "[" "identifier" "]" linebreaks
    {
        $$ = $2;
    }

itemlist:
    itemlist "identifier" "=" itemValue linebreaks
    {
        $1[$2] = $4;
        $$ = $1;
    }
|
    "identifier" "=" itemValue linebreaks
    {
        json newMap = json::object();
        newMap[$1] = $3;
        $$ = newMap;
    }

itemValue:
    "identifier" "=" "identifier"
    {
        std::string temp = $1 + "=" + $3;
        $$ = json(temp);
    }
|
    identifierlist
    {
        $$ = $1;
    }
|
    "identifier"
    {
        $$ = json($1);
    }
|
    "string_pln"
    {
        $$ = json($1);
    }
|
    "string"
    {
        $$ = json(driver.removeQuotes($1));
    }
 |
    "number"
    {
        $$ = json($1);
    }
|
    "float"
    {
        $$ = json($1);
    }
|
    "true"
    {
        $$ = json(true);
    }
|
    "false"
    {
        $$ = json(false);
    }
|
    %empty
    {
        $$ = json("");
    }


identifierlist:
    identifierlist "," "string"
    {
        $1.push_back(driver.removeQuotes($3));
        $$ = $1;
    }
|
    identifierlist "," "string_pln"
    {
        $1.push_back($3);
        $$ = $1;
    }
|
    identifierlist "," "identifier"
    {
        $1.push_back($3);
        $$ = $1;
    }
|
    "string" "," "string"
    {
        json tempItem = json::array();
        tempItem.push_back(driver.removeQuotes($1));
        tempItem.push_back(driver.removeQuotes($3));
        $$ = tempItem;
    }
|
    "string_pln" "," "string_pln"
    {
        json tempItem = json::array();
        tempItem.push_back($1);
        tempItem.push_back($3);
        $$ = tempItem;
    }
|
    "identifier" "," "identifier"
    {
        json tempItem = json::array();
        tempItem.push_back($1);
        tempItem.push_back($3);
        $$ = tempItem;
    }


linebreaks:
   linebreaks "lbreak"
|
   "lbreak"


%%

void Hanami::IniParser::error(const Hanami::location& location,
                                   const std::string& message)
{
    driver.error(location, message);
}
