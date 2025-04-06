/**
 * @file       cluster_parser.y
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
%require "3.8.2"

%define api.parser.class {ClusterParser}

%define api.prefix {cluster}
%define api.namespace {Hanami}
%define api.token.constructor
%define api.value.type variant
%define parse.assert

%code requires
{
#include <string>
#include <iostream>
#include <map>
#include <vector>

#include <hanami_cluster_parser/cluster_meta.h>
#include <hanami_common/structs.h>

namespace Hanami
{

class ClusterParserInterface;

}
}

// The parsing context.
%param { Hanami::ClusterParserInterface& driver }

%locations

%code
{
#include <cluster_parsing/cluster_parser_interface.h>
# undef YY_DECL
# define YY_DECL \
    Hanami::ClusterParser::symbol_type clusterlex (Hanami::ClusterParserInterface& driver)
YY_DECL;
}

// Token
%define api.token.prefix {Cluster_}
%token
    END  0  "end of file"
    COMMA  ","
    ASSIGN  ":"
    LINEBREAK "linebreak"
    VERSION_1 "version1"
    SETTINGS "settings"
    HEXAGONS "hexagons"
    INPUTS "inputs"
    OUTPUTS "outputs"
    AXONS "axons"
    COOLDOWN "neuron_cooldown"
    MAX_DISTANCE "max_connection_distance"
    REFRACTORY_TIME "refractory_time"
    BOOL_TRUE  "true"
    BOOL_FALSE "false"
    OUTPUT_BOOL "bool"
    OUTPUT_INT "int"
    OUTPUT_FLOAT "float"
    OPEN "("
    CLOSE ")"
    ARROW "->"
;

%token <std::string> IDENTIFIER "identifier"
%token <std::string> STRING "string"
%token <long> NUMBER "number"
%token <double> FLOAT_VAL "float_val"

%type  <Position> position
%type  <Position> hexagon
%type  <Hanami::InputMeta> input
%type  <Hanami::OutputMeta> output
%type  <Hanami::AxonMeta> axon

%%
%start startpoint;

// example
//
// version: 1
// settings:
//
// hexagons:
//     1,1,1
//     3,1,1
//     4,1,1
//
// axons:
//     1,1,1 -> 3,1,1
//
// inputs:
//     "input_hexagon": 1,1,1 (binary)
//
// outputs:
//     "output_hexagon": 4,1,1 (float)
//

startpoint:
    "version 1" "settings" ":" settings "hexagons" ":" hexagons "axons" ":" axons "inputs" ":" inputs "outputs" ":" outputs
    {
        driver.output->version = 1;
    }
|
    "version1" "settings" ":" settings "hexagons" ":" hexagons "axons" ":" axons "inputs" ":" inputs "outputs" ":" outputs
    {
        driver.output->version = 1;
    }
|
    "version 1" "settings" ":" settings "hexagons" ":" hexagons "inputs" ":" inputs "outputs" ":" outputs
    {
        driver.output->version = 1;
    }
|
    "version1" "settings" ":" settings "hexagons" ":" hexagons "inputs" ":" inputs "outputs" ":" outputs
    {
        driver.output->version = 1;
    }

settings:
    settings setting
    {}
|
    setting
    {}

setting:
    "neuron_cooldown" ":" "number"
    {
        if($3 <= 1) {
            driver.error(yyla.location, "neuron_cooldown must be > 1.0");
            return 1;
        }
        driver.output->neuronCooldown = $3;
    }
|
    "neuron_cooldown" ":" "float_val"
    {
        if($3 <= 1.0) {
            driver.error(yyla.location, "neuron_cooldown must be > 1.0");
            return 1;
        }
        driver.output->neuronCooldown = $3;
    }
|
    "max_connection_distance" ":" "number"
    {
        if($3 < 1) {
            driver.error(yyla.location, "max_connection_distance must be >= 1");
            return 1;
        }
        driver.output->maxConnectionDistance = $3;
    }
|
    "refractory_time" ":" "number"
    {
        if($3 < 1) {
            driver.error(yyla.location, "refractory_time must be >= 1");
            return 1;
        }
        driver.output->refractoryTime = $3;
    }

hexagons:
    hexagons hexagon
    {
        driver.output->hexagons.push_back($2);
    }
|
    hexagon
    {
        driver.output->hexagons.push_back($1);
    }

hexagon:
    position
    {
        const Hanami::Position pos = $1;
        const uint32_t hexagonId = driver.getHexagonId(pos);
        if(hexagonId != UNINTI_POINT_32) {
            driver.error(yyla.location, "Hexagon with the position " + pos.toString() + " already exist.");
            return 1;
        }
        $$ = $1;
    }

axons:
    axons axon
    {
        driver.output->axons.push_back($2);
    }
|
    axon
    {
        driver.output->axons.push_back($1);
    }

axon:
    position "->" position
    {
        const Hanami::Position sourcePos = $1;
        const uint32_t sourceHexagonId = driver.getHexagonId(sourcePos);
        if(sourceHexagonId == UNINTI_POINT_32) {
            driver.error(yyla.location, "Source-Hexagon of axon with the position " + sourcePos.toString() + " doesn't exist.");
            return 1;
        }

        const Hanami::Position targetPos = $3;
        const uint32_t targetHexagonId = driver.getHexagonId(targetPos);
        if(targetHexagonId == UNINTI_POINT_32) {
            driver.error(yyla.location, "Target-Hexagon of axon with the position " + targetPos.toString() + " doesn't exist.");
            return 1;
        }

        AxonMeta axon;
        axon.sourceId = sourceHexagonId;
        axon.targetId = targetHexagonId;
        $$ = axon;
    }

inputs:
    inputs input
    {
        driver.output->inputs.push_back($2);
    }
|
    input
    {
        driver.output->inputs.push_back($1);
    }

input:
    "identifier" ":" position
    {
        const Hanami::Position pos = $3;
        const uint32_t hexagonId = driver.getHexagonId(pos);
        if(hexagonId == UNINTI_POINT_32) {
            driver.error(yyla.location, "Hexagon with the position " + pos.toString() + " doesn't exist.");
            return 1;
        }
        if($1.size() >= 255) {
            driver.error(yyla.location, "Name '" + $1 + "' is longer than 254 characters.");
            return 1;
        }
        $$.name = $1;
        $$.targetHexagonId = hexagonId;
    }

outputs:
    outputs output
    {
        driver.output->outputs.push_back($2);
    }
|
    outputs output "(" "bool" ")"
    {
        $2.type = BOOL_OUTPUT;
        driver.output->outputs.push_back($2);
    }
|
    outputs output "(" "int" ")"
    {
        $2.type = INT_OUTPUT;
        driver.output->outputs.push_back($2);
    }
|
    outputs output "(" "float" ")"
    {
        $2.type = FLOAT_OUTPUT;
        driver.output->outputs.push_back($2);
    }
|
    output
    {
        driver.output->outputs.push_back($1);
    }
|
    output "(" "bool" ")"
    {
        $1.type = BOOL_OUTPUT;
        driver.output->outputs.push_back($1);
    }
|
    output "(" "int" ")"
    {
        $1.type = INT_OUTPUT;
        driver.output->outputs.push_back($1);
    }
|
    output "(" "float" ")"
    {
        $1.type = FLOAT_OUTPUT;
        driver.output->outputs.push_back($1);
    }


output:
    "identifier" ":" position
    {
        const Hanami::Position pos = $3;
        const uint32_t hexagonId = driver.getHexagonId(pos);
        if(hexagonId == UNINTI_POINT_32) {
            driver.error(yyla.location, "Hexagon with the position " + pos.toString() + " doesn't exist.");
            return 1;
        }
        if($1.size() >= 255) {
            driver.error(yyla.location, "Name '" + $1 + "' is longer than 254 characters.");
            return 1;
        }
        $$.name = $1;
        $$.targetHexagonId = hexagonId;
    }

position:
    "number" "," "number" "," "number"
    {
        Hanami::Position pos;
        pos.x = $1;
        pos.y = $3;
        pos.z = $5;
        $$ = pos;
    }
%%

void Hanami::ClusterParser::error(const Hanami::location& location,
                                  const std::string& message)
{
    driver.error(location, message);
}
