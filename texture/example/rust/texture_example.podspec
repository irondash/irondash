Pod::Spec.new do |s|
    s.name             = 'texture_example'
    s.version          = '0.0.1'
    s.summary          = 'Cocoa pod for building example rust library.'
    s.homepage         = 'http://example.com'
    s.author           = { 'Your Company' => 'email@example.com' }

    s.source           = { :path => '.' }
    s.source_files     = '*.c'

    s.ios.deployment_target = '11.0'
    s.macos.deployment_target = '10.13'

    # This will overwrite pod framework with rust dylib
    s.script_phase = {
      :name => 'Build Rust library',
      :script => 'sh $PODS_TARGET_SRCROOT/../cargokit/build_pod.sh ../rust texture_example',
      :execution_position=> :after_compile,
      :input_files => ['${BUILT_PRODUCTS_DIR}/cargokit_phony'],
      # Non existent file to force rebuild
      :output_files => ['${BUILT_PRODUCTS_DIR}/cargokit_phony_']
    }
end
