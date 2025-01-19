%{ /* -*- C++ -*- */
/**
 * @file       cluster_lexer.l
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

# include <cerrno>
# include <climits>
# include <cstdlib>
# include <string>
# include <cluster_parsing/cluster_parser_interface.h>
# include <cluster_parser.h>

# undef yywrap
# define yywrap() 1


// The location of the current token.
# ifdef YY_DECL
# undef YY_DECL
# endif
# define YY_DECL \
    Hanami::ClusterParser::symbol_type clusterlex (Hanami::ClusterParserInterface& driver)
YY_DECL;
Hanami::location clusterloc;

%}


%option noyywrap nounput batch debug yylineno prefix="cluster"

id    [a-zA-Z_0-9]*
long  -?([0-9]+)
double -?([0-9]+)\.([0-9]+)
blank [ \t]

%{
    # define YY_USER_ACTION  clusterloc.columns (yyleng);
%}

%%

%{
    clusterloc.step();
%}

{blank}+  clusterloc.step();
[\n]      {clusterloc.lines(1); clusterloc.step(); }
"version:1"               return Hanami::ClusterParser::make_VERSION_1 (clusterloc);
"version: 1"              return Hanami::ClusterParser::make_VERSION_1 (clusterloc);
"settings"                return Hanami::ClusterParser::make_SETTINGS (clusterloc);
"hexagons"                return Hanami::ClusterParser::make_HEXAGONS (clusterloc);
"inputs"                  return Hanami::ClusterParser::make_INPUTS (clusterloc);
"outputs"                 return Hanami::ClusterParser::make_OUTPUTS (clusterloc);
"axons"                   return Hanami::ClusterParser::make_AXONS (clusterloc);
"neuron_cooldown"         return Hanami::ClusterParser::make_COOLDOWN (clusterloc);
"max_connection_distance" return Hanami::ClusterParser::make_MAX_DISTANCE (clusterloc);
"refractory_time"         return Hanami::ClusterParser::make_REFRACTORY_TIME (clusterloc);
"enable_reduction"        return Hanami::ClusterParser::make_ENABLE_REDUCTION (clusterloc);
"true"                    return Hanami::ClusterParser::make_BOOL_TRUE (clusterloc);
"false"                   return Hanami::ClusterParser::make_BOOL_FALSE (clusterloc);
"True"                    return Hanami::ClusterParser::make_BOOL_TRUE (clusterloc);
"False"                   return Hanami::ClusterParser::make_BOOL_FALSE (clusterloc);
"bool"                    return Hanami::ClusterParser::make_OUTPUT_BOOL (clusterloc);
"int"                     return Hanami::ClusterParser::make_OUTPUT_INT (clusterloc);
"float"                   return Hanami::ClusterParser::make_OUTPUT_FLOAT (clusterloc);
","                       return Hanami::ClusterParser::make_COMMA (clusterloc);
":"                       return Hanami::ClusterParser::make_ASSIGN (clusterloc);
"("                       return Hanami::ClusterParser::make_OPEN (clusterloc);
")"                       return Hanami::ClusterParser::make_CLOSE (clusterloc);
"->"                      return Hanami::ClusterParser::make_ARROW (clusterloc);

\"(\\.|[^"\\])*\" { return Hanami::ClusterParser::make_STRING(yytext, clusterloc); }

{long}      {
    errno = 0;
    long length = strtol(yytext, NULL, 10);
    if (!(LONG_MIN <= length
        && length <= LONG_MAX
        && errno != ERANGE))
    {
        driver.error(clusterloc, "integer is out of range");
    }
    return Hanami::ClusterParser::make_NUMBER (length, clusterloc);
}

{double}	{
    double value = strtod( yytext , NULL );
    return Hanami::ClusterParser::make_FLOAT_VAL(value, clusterloc);
}

{id}       return Hanami::ClusterParser::make_IDENTIFIER(yytext, clusterloc);

.          driver.error(clusterloc, "invalid character");
<<EOF>>    return Hanami::ClusterParser::make_END(clusterloc);

%%


void Hanami::ClusterParserInterface::scan_begin(const std::string &inputString)
{
    Hanami::location newClusterloc;
    clusterloc = newClusterloc;
    yy_flex_debug = m_traceParsing;
    yy_scan_string(inputString.c_str());
}

void Hanami::ClusterParserInterface::scan_end()
{
    yy_delete_buffer(YY_CURRENT_BUFFER);
}


