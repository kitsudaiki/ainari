from setuptools import setup
from setuptools.command.install import install
from subprocess import check_call
import os

version = os.getenv('PYTHON_PACKAGE_VERSION', '0.0.0')


class GenerateProtobufMessages(install):
    def run(self):
        # Run your custom command here
        check_call(["protoc",
                    "--python_out=./hanami_sdk",
                    "--proto_path",
                    "../../../libs/protobuf",
                    "hanami_messages.proto3"])

        # Continue with the default installation process
        install.run(self)


setup(
    name='hanami_sdk',
    version=version,
    description='SDK library for OpenHanami',
    url='https://github.com/kitsudaiki/OpenHanami',
    author='Tobias Anker',
    author_email='tobias.anker@kitsunemimi.moe',
    license='Apache 2.0',
    packages=['hanami_sdk', 'hanami_sdk.hanami_messages'],
    install_requires=['jsonschema==4.24.0',
                      'protobuf==3.20.3',
                      'requests==2.32.4',
                      'simplejson==3.20.1',
                      'websockets==15.0.1',
                      'requests_toolbelt==1.0.0'],
    cmdclass={
        'install': GenerateProtobufMessages,
    },
    classifiers=[
        'License :: Apache 2.0',
        'Operating System :: POSIX :: Linux',
        'Programming Language :: Python :: 3.9',
        'Programming Language :: Python :: 3.10',
        'Programming Language :: Python :: 3.11',
        'Programming Language :: Python :: 3.12',
    ],
)
