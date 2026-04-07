from setuptools import setup
# from setuptools.command.install import install
# from subprocess import check_call
import os

version = os.getenv('PYTHON_PACKAGE_VERSION', '0.11.0')


# NOTE (kitsudaiki): leftover of old implementation, but left disabled
#                    here in the code for the case that protobuffer messages
#                    on client-side will be added in the future again
# class GenerateProtobufMessages(install):
#     def run(self):
#         # Run your custom command here
#         check_call(["protoc",
#                     "--python_out=./ainari_sdk",
#                     "--proto_path",
#                     "../../../libs/protobuf",
#                     "ainari_messages.proto3"])

#         # Continue with the default installation process
#         install.run(self)


setup(
    name='ainari_sdk',
    version=version,
    description='SDK library for Ainari',
    url='https://github.com/kitsudaiki/ainari',
    author='Tobias Anker',
    author_email='tobias.anker@kitsunemimi.moe',
    license='Apache 2.0',
    # packages=['ainari_sdk', 'ainari_sdk.ainari_messages'],
    packages=['ainari_sdk'],
    install_requires=['jsonschema==4.26.0',
                      'requests==2.33.0',
                      'simplejson==3.20.2',
                      'requests_toolbelt==1.0.0'],
    # cmdclass={
    #     'install': GenerateProtobufMessages,
    # },
    classifiers=[
        'License :: Apache 2.0',
        'Operating System :: POSIX :: Linux',
        'Programming Language :: Python :: 3.9',
        'Programming Language :: Python :: 3.10',
        'Programming Language :: Python :: 3.11',
        'Programming Language :: Python :: 3.12',
    ],
)
