%{ /* -*- C++ -*-*/
/**
 * @file    ini_lexer.l
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
# include <ini_parsing/ini_parser_interface.h>
# include <ini_parser.h>

# undef yywrap
# define yywrap() 1

// The location of the current token.
# ifdef YY_DECL
# undef YY_DECL
# endif
# define YY_DECL \
    Hanami::IniParser::symbol_type inilex (Hanami::IniParserInterface& driver)
YY_DECL;

Hanami::location iniloc;
%}


%option noyywrap nounput batch debug yylineno prefix="ini"
id    [%\-+\/@.:a-zA-Z_0-9]*
long   -?([0-9]+)
double -?([0-9]+)\.([0-9]+)
blank [ \t]

%{
    # define YY_USER_ACTION  iniloc.columns (yyleng);
%}

%%

%{
    iniloc.step();
%}

#.*$        iniloc.step();
{blank}+    iniloc.step();
[\n]        {iniloc.lines(1); return Hanami::IniParser::make_LINEBREAK (iniloc); }
"true"      return Hanami::IniParser::make_BOOL_TRUE (iniloc);
"false"     return Hanami::IniParser::make_BOOL_FALSE (iniloc);
"True"      return Hanami::IniParser::make_BOOL_TRUE (iniloc);
"False"     return Hanami::IniParser::make_BOOL_FALSE (iniloc);
"="         return Hanami::IniParser::make_EQUAL (iniloc);
"["         return Hanami::IniParser::make_BRACKOPEN (iniloc);
"]"         return Hanami::IniParser::make_BRACKCLOSE (iniloc);
","         return Hanami::IniParser::make_COMMA (iniloc);
\"(\\.|[^"\\])*\" {
            return Hanami::IniParser::make_STRING(yytext, iniloc);
}


{long}      {
    errno = 0;
    long length = strtol(yytext, NULL, 10);
    if (!(LONG_MIN <= length
        && length <= LONG_MAX
        && errno != ERANGE))
    {
        driver.error(iniloc, "integer is out of range");
    }
    return Hanami::IniParser::make_NUMBER (length, iniloc);
}

{double}	{
    double value = strtod( yytext , NULL );
    return Hanami::IniParser::make_FLOAT(value, iniloc);
}

{id} return Hanami::IniParser::make_IDENTIFIER(yytext, iniloc);


[a-zA-Z_0-9|\-|.]* {
    return Hanami::IniParser::make_STRING_PLN(yytext, iniloc);
}

.          driver.error(iniloc, "invalid character");
<<EOF>>    return Hanami::IniParser::make_END(iniloc);

%%


void Hanami::IniParserInterface::scan_begin(const std::string &inputString)
{
    Hanami::location newLoc;
    iniloc = newLoc;
    yy_flex_debug = m_traceParsing;
    yy_scan_string(inputString.c_str());
}

void Hanami::IniParserInterface::scan_end()
{
    yy_delete_buffer(YY_CURRENT_BUFFER);
}
