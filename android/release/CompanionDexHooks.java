import java.io.File;
import java.util.*;
import org.jf.dexlib2.*;
import org.jf.dexlib2.iface.*;
import org.jf.dexlib2.iface.instruction.ReferenceInstruction;
import org.jf.dexlib2.iface.instruction.FiveRegisterInstruction;
import org.jf.dexlib2.iface.reference.MethodReference;
import org.jf.dexlib2.immutable.*;
import org.jf.dexlib2.immutable.reference.ImmutableMethodReference;
import org.jf.dexlib2.builder.*;
import org.jf.dexlib2.builder.instruction.*;

/** Replace the old companion wheel entry points with the source-built controller. */
public final class CompanionDexHooks {
    public static void main(String[] args) throws Exception {
        DexFile dex=DexFileFactory.loadDexFile(new File(args[0]),Opcodes.forApi(26));
        List<ClassDef> classes=new ArrayList<>(); int hooks=0,recovery=0,removedWheel=0,removedSelect=0;
        for(ClassDef cls:dex.getClasses()) {
            if(cls.getType().equals("Lorg/rerac/android/WeaponWheel$1;")){
                List<Method> methods=new ArrayList<>();
                for(Method method:cls.getMethods()){
                    if(!method.getName().equals("run")){methods.add(method);continue;}
                    MutableMethodImplementation code=new MutableMethodImplementation(method.getImplementation());
                    int writes=0;
                    for(int i=0;i<code.getInstructions().size();i++){
                        BuilderInstruction insn=code.getInstructions().get(i);
                        if(!(insn instanceof ReferenceInstruction))continue;
                        Object ref=((ReferenceInstruction)insn).getReference();
                        if(!(ref instanceof MethodReference))continue;
                        MethodReference target=(MethodReference)ref;
                        if(!target.getDefiningClass().equals("Lorg/rerac/android/WeaponWheel;")||!target.getName().equals("-$$Nest$fputenabled"))continue;
                        // This input DEX is SHA-pinned by companion_dex.py.
                        // The first write applies the parsed packet's enabled
                        // flag; the second is the shared exception handler.
                        if(++writes!=2)continue;
                        FiveRegisterInstruction argsInsn=(FiveRegisterInstruction)insn;
                        code.replaceInstruction(i,new BuilderInstruction35c(Opcode.INVOKE_STATIC,2,argsInsn.getRegisterC(),argsInsn.getRegisterD(),0,0,0,
                            new ImmutableMethodReference("Lorg/rerac/android/CompanionMenu;","pollFailed",Arrays.asList("Landroid/view/View;","Z"),"V")));
                        recovery++;
                    }
                    if(writes!=2)throw new IllegalStateException("Unexpected retained poll enabled writes: "+writes);
                    methods.add(new ImmutableMethod(method.getDefiningClass(),method.getName(),method.getParameters(),method.getReturnType(),method.getAccessFlags(),method.getAnnotations(),method.getHiddenApiRestrictions(),code));
                }
                classes.add(new ImmutableClassDef(cls.getType(),cls.getAccessFlags(),cls.getSuperclass(),cls.getInterfaces(),cls.getSourceFile(),cls.getAnnotations(),cls.getFields(),methods));
                continue;
            }
            if(!cls.getType().equals("Lorg/rerac/android/WeaponWheel;")){classes.add(cls);continue;}
            List<Method> methods=new ArrayList<>();
            for(Method method:cls.getMethods()) {
                if(method.getName().equals("quickSelect")&&method.getParameterTypes().equals(Arrays.asList("Landroid/graphics/Canvas;","Landroid/graphics/RectF;"))){removedWheel++;continue;}
                boolean draw=method.getName().equals("onDraw")&&method.getParameterTypes().equals(Arrays.asList("Landroid/graphics/Canvas;"));
                boolean touch=method.getName().equals("onTouchEvent")&&method.getParameterTypes().equals(Arrays.asList("Landroid/view/MotionEvent;"));
                if(!draw&&!touch){methods.add(method);continue;}
                if(touch){
                    // Complex pause pages retain their original widget routing.
                    // Delete the old equipment-send instruction even here: the
                    // new controller is the only companion equipment producer.
                    MutableMethodImplementation old=new MutableMethodImplementation(method.getImplementation());
                    for(int i=0;i<old.getInstructions().size();i++){
                        BuilderInstruction insn=old.getInstructions().get(i);
                        if(!(insn instanceof ReferenceInstruction))continue;
                        Object ref=((ReferenceInstruction)insn).getReference();if(!(ref instanceof MethodReference))continue;
                        MethodReference target=(MethodReference)ref;
                        if(target.getDefiningClass().equals("Lorg/rerac/android/GameActivity;")&&target.getName().equals("nativeSelect")){
                            old.replaceInstruction(i,new BuilderInstruction10x(Opcode.NOP));removedSelect++;
                        }
                    }
                    methods.add(new ImmutableMethod(method.getDefiningClass(),"retainedMenuTouch",method.getParameters(),method.getReturnType(),method.getAccessFlags(),method.getAnnotations(),method.getHiddenApiRestrictions(),old));
                }
                MutableMethodImplementation code=new MutableMethodImplementation(draw?2:3);
                code.addInstruction(new BuilderInstruction3rc(Opcode.INVOKE_STATIC_RANGE,draw?0:1,2,
                    new ImmutableMethodReference("Lorg/rerac/android/CompanionQuickSelect;",draw?"draw":"touch",
                    Arrays.asList("Landroid/view/View;",draw?"Landroid/graphics/Canvas;":"Landroid/view/MotionEvent;"),draw?"V":"Z")));
                if(draw)code.addInstruction(new BuilderInstruction10x(Opcode.RETURN_VOID));
                else{code.addInstruction(new BuilderInstruction11x(Opcode.MOVE_RESULT,0));code.addInstruction(new BuilderInstruction11x(Opcode.RETURN,0));}
                methods.add(new ImmutableMethod(method.getDefiningClass(),method.getName(),method.getParameters(),method.getReturnType(),method.getAccessFlags(),method.getAnnotations(),method.getHiddenApiRestrictions(),code));
                hooks++;
            }
            classes.add(new ImmutableClassDef(cls.getType(),cls.getAccessFlags(),cls.getSuperclass(),cls.getInterfaces(),cls.getSourceFile(),cls.getAnnotations(),cls.getFields(),methods));
        }
        if(hooks!=2)throw new IllegalStateException("Expected exactly two retained companion hooks; got "+hooks);
        if(recovery!=1)throw new IllegalStateException("Expected one retained poll recovery hook; got "+recovery);
        if(removedWheel!=1||removedSelect!=1)throw new IllegalStateException("Old quick-select code was not completely disconnected: "+removedWheel+" / "+removedSelect);
        DexFileFactory.writeDexFile(args[1],new ImmutableDexFile(Opcodes.forApi(26),classes));
    }
}
